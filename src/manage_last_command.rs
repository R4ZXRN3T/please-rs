use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, error::Error, fs, io};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellKind {
	Cmd,
	PowerShell,
	Bash,
	Sh,
	Zsh,
	Fish,
	Nu,
	Unknown,
}

pub fn get_last_command() -> Result<Vec<String>, Box<dyn Error>> {
	let detected = detect_shell();
	if detected == ShellKind::Unknown {
		return Err(io::Error::other(
			"unable to detect active shell; set PLEASE_SHELL or pass a command explicitly (e.g. please <cmd>)",
		)
			.into());
	}

	let mut skip = 0usize;
	loop {
		let Some(raw_command) = get_last_command_for_shell(detected, skip)? else {
			return Err(io::Error::other(
				"unable to discover a previous non-'please' command for detected shell; pass a command explicitly (e.g. please <cmd>)",
			)
				.into());
		};

		if !is_self_invocation(&raw_command) {
			return Ok(shell_command_to_exec(detected, raw_command));
		}

		skip += 1;
	}
}

fn detect_shell() -> ShellKind {
	if let Some(shell) = env::var_os("PLEASE_SHELL") {
		return parse_shell_name(&shell.to_string_lossy());
	}

	if let Some(shell_path) = env::var_os("SHELL") {
		if let Some(name) = Path::new(&shell_path).file_name() {
			let parsed = parse_shell_name(&name.to_string_lossy());
			if parsed != ShellKind::Unknown {
				return parsed;
			}
		}
	}

	if env::var_os("NU_VERSION").is_some() {
		return ShellKind::Nu;
	}

	if env::var_os("PSModulePath").is_some() {
		return ShellKind::PowerShell;
	}

	if cfg!(windows) {
		if let Some(comspec) = env::var_os("ComSpec") {
			if comspec
				.to_string_lossy()
				.to_ascii_lowercase()
				.contains("cmd.exe")
			{
				return ShellKind::Cmd;
			}
		}
	}

	ShellKind::Unknown
}

fn parse_shell_name(value: &str) -> ShellKind {
	let shell_name = Path::new(value)
		.file_name()
		.and_then(|name| name.to_str())
		.unwrap_or(value)
		.to_ascii_lowercase();

	match shell_name.as_str() {
		"cmd" | "cmd.exe" => ShellKind::Cmd,
		"powershell" | "powershell.exe" | "pwsh" | "pwsh.exe" => ShellKind::PowerShell,
		"bash" => ShellKind::Bash,
		"sh" => ShellKind::Sh,
		"zsh" => ShellKind::Zsh,
		"fish" => ShellKind::Fish,
		"nu" | "nushell" => ShellKind::Nu,
		_ => ShellKind::Unknown,
	}
}
fn get_last_command_for_shell(
	shell: ShellKind,
	skip: usize,
) -> Result<Option<String>, Box<dyn Error>> {
	match shell {
		ShellKind::PowerShell => read_powershell_history(skip),
		ShellKind::Cmd => read_cmd_history(skip),
		ShellKind::Bash => read_bash_like_history("HISTFILE", ".bash_history", skip),
		ShellKind::Sh => read_bash_like_history("HISTFILE", ".sh_history", skip),
		ShellKind::Zsh => read_zsh_history(skip),
		ShellKind::Fish => read_fish_history(skip),
		ShellKind::Nu => read_nu_history(skip),
		ShellKind::Unknown => Ok(None),
	}
}

fn shell_command_to_exec(shell: ShellKind, raw_command: String) -> Vec<String> {
	match shell {
		ShellKind::Cmd => vec!["cmd".to_owned(), "/C".to_owned(), raw_command],
		ShellKind::PowerShell => {
			vec![
				powershell_program().to_owned(),
				"-NoProfile".to_owned(),
				"-Command".to_owned(),
				raw_command,
			]
		}
		ShellKind::Bash => vec!["bash".to_owned(), "-c".to_owned(), raw_command],
		ShellKind::Sh => vec!["sh".to_owned(), "-c".to_owned(), raw_command],
		ShellKind::Zsh => vec!["zsh".to_owned(), "-c".to_owned(), raw_command],
		ShellKind::Fish => vec!["fish".to_owned(), "-c".to_owned(), raw_command],
		ShellKind::Nu => vec!["nu".to_owned(), "-c".to_owned(), raw_command],
		ShellKind::Unknown => vec![raw_command],
	}
}

fn powershell_program() -> &'static str {
	if cfg!(windows) { "powershell" } else { "pwsh" }
}

fn get_home_dir() -> Option<PathBuf> {
	if let Some(home) = env::var_os("HOME") {
		return Some(PathBuf::from(home));
	}

	if let Some(profile) = env::var_os("USERPROFILE") {
		return Some(PathBuf::from(profile));
	}

	let drive = env::var_os("HOMEDRIVE");
	let path = env::var_os("HOMEPATH");
	match (drive, path) {
		(Some(d), Some(p)) => {
			let mut out = PathBuf::from(d);
			out.push(p);
			Some(out)
		}
		_ => None,
	}
}

fn nth_from_latest<I>(items: I, skip: usize) -> Option<String>
where
	I: IntoIterator<Item=String>,
{
	items.into_iter().nth(skip)
}

fn read_nth_non_empty_line(path: &Path, skip: usize) -> Result<Option<String>, Box<dyn Error>> {
	let content = match fs::read_to_string(path) {
		Ok(content) => content,
		Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
		Err(err) => {
			return Err(io::Error::other(format!(
				"failed to read history file '{}': {err}",
				path.display()
			))
				.into());
		}
	};

	let candidates = content
		.lines()
		.rev()
		.map(str::trim)
		.filter(|line| !line.is_empty())
		.map(ToOwned::to_owned)
		.collect::<Vec<_>>();

	Ok(nth_from_latest(candidates, skip))
}

fn read_powershell_history(skip: usize) -> Result<Option<String>, Box<dyn Error>> {
	if let Some(app_data) = env::var_os("APPDATA") {
		let mut history_path = PathBuf::from(app_data);
		history_path.push("Microsoft");
		history_path.push("Windows");
		history_path.push("PowerShell");
		history_path.push("PSReadLine");
		history_path.push("ConsoleHost_history.txt");
		return read_nth_non_empty_line(&history_path, skip);
	}

	Ok(None)
}

fn read_cmd_history(skip: usize) -> Result<Option<String>, Box<dyn Error>> {
	let output = Command::new("cmd")
		.args(["/C", "doskey", "/history"])
		.output();

	let output = match output {
		Ok(value) => value,
		Err(_) => return Ok(None),
	};

	if !output.status.success() {
		return Ok(None);
	}

	let text = String::from_utf8_lossy(&output.stdout);
	let candidates = text
		.lines()
		.rev()
		.map(str::trim)
		.filter(|line| !line.is_empty())
		.map(ToOwned::to_owned)
		.collect::<Vec<_>>();

	Ok(nth_from_latest(candidates, skip))
}

fn read_bash_like_history(
	histfile_env: &str,
	default_file: &str,
	skip: usize,
) -> Result<Option<String>, Box<dyn Error>> {
	let path = if let Some(histfile) = env::var_os(histfile_env) {
		PathBuf::from(histfile)
	} else if let Some(home) = get_home_dir() {
		home.join(default_file)
	} else {
		return Ok(None);
	};

	let content = match fs::read_to_string(&path) {
		Ok(content) => content,
		Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
		Err(err) => {
			return Err(io::Error::other(format!(
				"failed to read history file '{}': {err}",
				path.display()
			))
				.into());
		}
	};

	let candidates = content
		.lines()
		.rev()
		.map(str::trim)
		.filter(|line| !line.is_empty())
		.filter(|line| !is_bash_timestamp_marker(line))
		.map(ToOwned::to_owned)
		.collect::<Vec<_>>();

	Ok(nth_from_latest(candidates, skip))
}

fn is_bash_timestamp_marker(line: &str) -> bool {
	line.starts_with('#') && line[1..].chars().all(|ch| ch.is_ascii_digit())
}

fn read_zsh_history(skip: usize) -> Result<Option<String>, Box<dyn Error>> {
	let Some(home) = get_home_dir() else {
		return Ok(None);
	};

	let path = home.join(".zsh_history");
	let content = match fs::read_to_string(&path) {
		Ok(content) => content,
		Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
		Err(err) => {
			return Err(io::Error::other(format!(
				"failed to read history file '{}': {err}",
				path.display()
			))
				.into());
		}
	};

	let candidates = content
		.lines()
		.rev()
		.map(str::trim)
		.filter(|line| !line.is_empty())
		.filter_map(|line| {
			if let Some(after_semicolon) = line.split_once(';').map(|(_, right)| right.trim()) {
				if !after_semicolon.is_empty() {
					return Some(after_semicolon.to_owned());
				}
			}

			Some(line.to_owned())
		})
		.collect::<Vec<_>>();

	Ok(nth_from_latest(candidates, skip))
}

fn read_fish_history(skip: usize) -> Result<Option<String>, Box<dyn Error>> {
	let path = if let Some(xdg_data_home) = env::var_os("XDG_DATA_HOME") {
		PathBuf::from(xdg_data_home)
			.join("fish")
			.join("fish_history")
	} else if let Some(home) = get_home_dir() {
		home.join(".local")
			.join("share")
			.join("fish")
			.join("fish_history")
	} else {
		return Ok(None);
	};

	let content = match fs::read_to_string(&path) {
		Ok(content) => content,
		Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
		Err(err) => {
			return Err(io::Error::other(format!(
				"failed to read history file '{}': {err}",
				path.display()
			))
				.into());
		}
	};

	let candidates = content
		.lines()
		.rev()
		.map(str::trim)
		.filter_map(|line| line.strip_prefix("- cmd: ").map(str::trim))
		.filter(|command| !command.is_empty())
		.map(ToOwned::to_owned)
		.collect::<Vec<_>>();

	Ok(nth_from_latest(candidates, skip))
}

fn read_nu_history(skip: usize) -> Result<Option<String>, Box<dyn Error>> {
	let nu_query = format!("history | get command | reverse | skip {} | first", skip);
	let output = Command::new("nu").args(["-c", &nu_query]).output();

	if let Ok(value) = output {
		if value.status.success() {
			let from_cli = String::from_utf8_lossy(&value.stdout).trim().to_owned();
			if !from_cli.is_empty() {
				return Ok(Some(from_cli));
			}
		}
	}

	let Some(home) = get_home_dir() else {
		return Ok(None);
	};

	let text_history_path = home.join(".config").join("nushell").join("history.txt");
	read_nth_non_empty_line(&text_history_path, skip)
}

fn is_self_invocation(command_line: &str) -> bool {
	let first = command_line.split_whitespace().next().unwrap_or_default();
	let first = first
		.trim_matches('"')
		.trim_matches('\'')
		.to_ascii_lowercase();

	first == "please"
		|| first == "please.exe"
		|| first.ends_with("/please")
		|| first.ends_with("/please.exe")
		|| first.ends_with("\\please")
		|| first.ends_with("\\please.exe")
}
