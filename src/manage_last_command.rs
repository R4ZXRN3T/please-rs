use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::{env, error::Error, fmt, fs, io};

/// Represents the type of shell being used.
///
/// Used for determining how to read shell history and format commands for execution.
/// Each variant corresponds to a different shell environment with its own history format
/// and command execution requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
	/// Windows Command Prompt (cmd.exe)
	Cmd,
	/// PowerShell (both Windows and Core versions)
	PowerShell,
	/// GNU Bash shell
	Bash,
	/// POSIX shell (sh)
	Sh,
	/// Z shell (Zsh)
	Zsh,
	/// Friendly interactive shell (Fish)
	Fish,
	/// Nushell (Nu)
	Nu,
	/// Unknown or undetected shell
	Unknown
}

impl ShellKind {
	pub fn as_str(self) -> &'static str {
		match self {
			ShellKind::Cmd => "cmd",
			ShellKind::PowerShell => "powershell",
			ShellKind::Bash => "bash",
			ShellKind::Sh => "sh",
			ShellKind::Zsh => "zsh",
			ShellKind::Fish => "fish",
			ShellKind::Nu => "nu",
			ShellKind::Unknown => "unknown",
		}
	}
}

impl fmt::Display for ShellKind {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

/// Retrieves the last non-'please' command from shell history and converts it to an executable format.
///
/// Detects the active shell, searches through history skipping any 'please' invocations,
/// and returns the command formatted for execution with the detected shell.
///
/// # Returns
///
/// Returns a vector of strings representing the command and its arguments.
///
/// # Errors
///
/// Returns an error if the shell cannot be detected or if no suitable command is found in history.
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

/// Detects the currently active shell environment.
///
/// Checks environment variables and system configuration to determine the active shell.
/// Supports: PowerShell, Cmd, Bash, Sh, Zsh, Fish, and Nushell.
///
/// # Returns
///
/// Returns a `ShellKind` enum representing the detected shell, or `ShellKind::Unknown` if detection fails.
pub(crate) fn detect_shell() -> ShellKind {
	if let Some(shell) = env::var_os("PLEASE_SHELL") {
		return parse_shell_name(&shell.to_string_lossy());
	}

	if let Some(shell) = detect_shell_from_parent_process() {
		return shell;
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

/// Attempts to detect the shell from the current process parent chain.
///
/// This helps prefer the currently active interactive shell over login/default shell variables.
fn detect_shell_from_parent_process() -> Option<ShellKind> {
	if cfg!(windows) {
		detect_shell_from_parent_process_windows()
	} else {
		detect_shell_from_parent_process_unix()
	}
}

/// Detects a shell by traversing parent processes via `ps` on Unix-like systems.
fn detect_shell_from_parent_process_unix() -> Option<ShellKind> {
	let mut pid = process::id();

	for _ in 0..8 {
		let parent_pid = unix_parent_pid(pid)?;
		let process_name = unix_process_name(parent_pid)?;
		let parsed = parse_shell_name(&process_name);
		if parsed != ShellKind::Unknown {
			return Some(parsed);
		}
		pid = parent_pid;
	}

	None
}

fn unix_parent_pid(pid: u32) -> Option<u32> {
	let output = Command::new("ps")
		.args(["-o", "ppid=", "-p", &pid.to_string()])
		.output()
		.ok()?;

	if !output.status.success() {
		return None;
	}

	String::from_utf8_lossy(&output.stdout)
		.trim()
		.parse::<u32>()
		.ok()
}

fn unix_process_name(pid: u32) -> Option<String> {
	let output = Command::new("ps")
		.args(["-o", "comm=", "-p", &pid.to_string()])
		.output()
		.ok()?;

	if !output.status.success() {
		return None;
	}

	let name = String::from_utf8_lossy(&output.stdout).trim().to_owned();
	if name.is_empty() {
		None
	} else {
		Some(name)
	}
}

/// Detects a shell by traversing parent processes through WMI on Windows.
fn detect_shell_from_parent_process_windows() -> Option<ShellKind> {
	let mut pid = process::id();

	for _ in 0..8 {
		let parent_pid = windows_parent_pid(pid)?;
		let process_name = windows_process_name(parent_pid)?;
		let parsed = parse_shell_name(&process_name);
		if parsed != ShellKind::Unknown {
			return Some(parsed);
		}
		pid = parent_pid;
	}

	None
}

fn windows_parent_pid(pid: u32) -> Option<u32> {
	let script = format!(
		"(Get-CimInstance Win32_Process -Filter \"ProcessId={}\").ParentProcessId",
		pid
	);
	let output = run_powershell_query(&script)?;
	output.trim().parse::<u32>().ok()
}

fn windows_process_name(pid: u32) -> Option<String> {
	let script = format!("(Get-CimInstance Win32_Process -Filter \"ProcessId={}\").Name", pid);
	let name = run_powershell_query(&script)?;
	let name = name.trim();
	if name.is_empty() {
		None
	} else {
		Some(name.to_owned())
	}
}

fn run_powershell_query(script: &str) -> Option<String> {
	for program in ["pwsh", "powershell"] {
		let output = Command::new(program)
			.args(["-NoProfile", "-Command", script])
			.output();

		if let Ok(value) = output {
			if value.status.success() {
				let text = String::from_utf8_lossy(&value.stdout).trim().to_owned();
				if !text.is_empty() {
					return Some(text);
				}
			}
		}
	}

	None
}

/// Parses a shell name string into a `ShellKind` enum variant.
///
/// Extracts the executable name from a full path and matches it against known shell names.
/// The matching is case-insensitive.
///
/// # Arguments
///
/// * `value` - A string containing a shell name or full path to a shell executable.
///
/// # Returns
///
/// Returns the matched `ShellKind`, or `ShellKind::Unknown` if no match is found.
fn parse_shell_name(value: &str) -> ShellKind {
	let shell_name = Path::new(value)
		.file_name()
		.and_then(|name| name.to_str())
		.unwrap_or(value)
		.to_ascii_lowercase();
	let shell_name = shell_name.trim_start_matches('-');

	match shell_name {
		"cmd" | "cmd.exe" => ShellKind::Cmd,
		"powershell" | "powershell.exe" | "pwsh" | "pwsh.exe" | "pwsh-preview"
		| "pwsh-preview.exe" => ShellKind::PowerShell,
		"bash" | "bash.exe" => ShellKind::Bash,
		"sh" | "sh.exe" | "dash" | "dash.exe" | "ksh" | "ksh.exe" | "ash" | "ash.exe" => {
			ShellKind::Sh
		}
		"zsh" | "zsh.exe" => ShellKind::Zsh,
		"fish" | "fish.exe" => ShellKind::Fish,
		"nu" | "nu.exe" | "nushell" | "nushell.exe" => ShellKind::Nu,
		_ => ShellKind::Unknown,
	}
}
/// Retrieves the last command from the history of a specific shell.
///
/// Dispatches to the appropriate shell-specific history reading function
/// and returns the n-th command from the latest, where n is the `skip` value.
///
/// # Arguments
///
/// * `shell` - The type of shell to retrieve history from.
/// * `skip` - The number of commands to skip from the latest (0 = most recent).
///
/// # Returns
///
/// Returns an option containing the command string, or `None` if no command is found.
///
/// # Errors
///
/// Returns an error if there's a failure reading the history file or executing shell commands.
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

/// Converts a raw command string into a shell-specific executable command format.
///
/// Wraps the command with the appropriate shell invocation flags based on the detected shell type.
///
/// # Arguments
///
/// * `shell` - The type of shell to format the command for.
/// * `raw_command` - The raw command string to be executed.
///
/// # Returns
///
/// Returns a vector of strings representing the shell executable and arguments needed to execute the command.
fn shell_command_to_exec(shell: ShellKind, raw_command: String) -> Vec<String> {
	match shell {
		ShellKind::Cmd => vec!["cmd".to_owned(), "/C".to_owned(), raw_command],
		ShellKind::PowerShell => {
			vec![
				powershell_program(),
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

/// Returns the appropriate PowerShell executable name for the current platform.
///
/// On Windows, tries to use `pwsh` (PowerShell 6+) if available, falling back to
/// `powershell` (PowerShell 5.1 or earlier) if not found.
/// On Unix-like systems, always returns `pwsh`.
///
/// # Returns
///
/// Returns the PowerShell executable name as a string.
fn powershell_program() -> String {
	if cfg!(windows) {
		// On Windows, try pwsh (PowerShell 6+) first, then fall back to powershell (5.1)
		if Command::new("pwsh")
			.args(["-NoProfile", "-Command", "exit 0"])
			.output()
			.is_ok()
		{
			"pwsh".to_owned()
		} else {
			"powershell".to_owned()
		}
	} else {
		"pwsh".to_owned()
	}
}

/// Retrieves the user's home directory.
///
/// Checks multiple environment variables and platform-specific paths to find the home directory.
/// Supports HOME, USERPROFILE, and Windows-specific HOMEDRIVE/HOMEPATH variables.
///
/// # Returns
///
/// Returns `Some(PathBuf)` containing the home directory path, or `None` if not found.
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

/// Retrieves the nth element from an iterator, skipping the first `skip` elements.
///
/// # Arguments
///
/// * `items` - An iterator of strings.
/// * `skip` - The number of items to skip from the beginning (0 = first item).
///
/// # Returns
///
/// Returns `Some(String)` containing the nth element, or `None` if not enough elements exist.
fn nth_from_latest<I>(items: I, skip: usize) -> Option<String>
where
	I: IntoIterator<Item=String>,
{
	items.into_iter().nth(skip)
}

/// Reads the nth non-empty line from a file in reverse order.
///
/// # Arguments
///
/// * `path` - The path to the file to read.
/// * `skip` - The number of non-empty lines to skip from the end (0 = last line).
///
/// # Returns
///
/// Returns `Some(String)` containing the requested line, or `None` if the file is not found
/// or doesn't contain enough non-empty lines.
///
/// # Errors
///
/// Returns an error if reading the file fails (except for NotFound errors).
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

/// Reads the PowerShell history from the PSReadLine history file.
///
/// # Arguments
///
/// * `skip` - The number of history entries to skip from the most recent (0 = most recent).
///
/// # Returns
///
/// Returns `Some(String)` containing the requested command, or `None` if the history file is not found
/// or the APPDATA environment variable is not set.
///
/// # Errors
///
/// Returns an error if reading the history file fails.
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

/// Reads the command history from the Windows cmd.exe doskey history.
///
/// # Arguments
///
/// * `skip` - The number of history entries to skip from the most recent (0 = most recent).
///
/// # Returns
///
/// Returns `Some(String)` containing the requested command, or `None` if the doskey command fails.
///
/// # Errors
///
/// Returns an error if reading the history fails for reasons other than the command not being found.
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

/// Reads the command history from bash-like shell history files.
///
/// Supports both custom HISTFILE environment variable paths and default history file locations.
/// Automatically filters out bash timestamp markers.
///
/// # Arguments
///
/// * `histfile_env` - The environment variable name for the history file path (e.g., "HISTFILE").
/// * `default_file` - The default history file name relative to home directory (e.g., ".bash_history").
/// * `skip` - The number of history entries to skip from the most recent (0 = most recent).
///
/// # Returns
///
/// Returns `Some(String)` containing the requested command, or `None` if the history file is not found.
///
/// # Errors
///
/// Returns an error if reading the history file fails for reasons other than NotFound.
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

/// Checks if a line is a bash timestamp marker.
///
/// Bash history files can contain timestamp markers in the format `#<digits>`.
/// This function identifies such markers.
///
/// # Arguments
///
/// * `line` - The line to check.
///
/// # Returns
///
/// Returns `true` if the line is a bash timestamp marker, `false` otherwise.
fn is_bash_timestamp_marker(line: &str) -> bool {
	line.starts_with('#') && line[1..].chars().all(|ch| ch.is_ascii_digit())
}

/// Reads the command history from a Zsh history file.
///
/// Zsh history entries may include timing information separated by semicolons.
/// This function parses those entries and extracts the actual commands.
///
/// # Arguments
///
/// * `skip` - The number of history entries to skip from the most recent (0 = most recent).
///
/// # Returns
///
/// Returns `Some(String)` containing the requested command, or `None` if the history file is not found.
///
/// # Errors
///
/// Returns an error if reading the history file fails for reasons other than NotFound.
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

/// Reads the command history from a Fish shell history file.
///
/// Fish history is stored in YAML format with commands prefixed by "- cmd: ".
/// This function extracts the command entries and filters timestamps.
///
/// # Arguments
///
/// * `skip` - The number of history entries to skip from the most recent (0 = most recent).
///
/// # Returns
///
/// Returns `Some(String)` containing the requested command, or `None` if the history file is not found.
///
/// # Errors
///
/// Returns an error if reading the history file fails for reasons other than NotFound.
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

/// Reads the command history from a Nushell history source.
///
/// Attempts to retrieve history through the `nu` command first.
/// If that fails, falls back to reading the text history file.
///
/// # Arguments
///
/// * `skip` - The number of history entries to skip from the most recent (0 = most recent).
///
/// # Returns
///
/// Returns `Some(String)` containing the requested command, or `None` if no history is found.
///
/// # Errors
///
/// Returns an error if reading the history file fails for reasons other than NotFound.
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

/// Checks if a command line is a self-invocation of the `please` command.
///
/// Detects various forms of the `please` command including direct invocations,
/// executable names, and path-based references with forward or backward slashes.
///
/// # Arguments
///
/// * `command_line` - The command line string to check.
///
/// # Returns
///
/// Returns `true` if the command is a `please` invocation, `false` otherwise.
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
