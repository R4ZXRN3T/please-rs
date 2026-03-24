use crate::manage_last_command;
use crate::manage_last_command::ShellKind;
use std::path::Path;
use std::process::Command;
use std::{env, process};

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
		return manage_last_command::parse_shell_name(&shell.to_string_lossy());
	}

	if let Some(shell) = detect_shell_from_parent_process() {
		return shell;
	}

	if let Some(shell_path) = env::var_os("SHELL")
		&& let Some(name) = Path::new(&shell_path).file_name()
	{
		let parsed = manage_last_command::parse_shell_name(&name.to_string_lossy());
		if parsed != ShellKind::Unknown {
			return parsed;
		}
	}

	if env::var_os("NU_VERSION").is_some() {
		return ShellKind::Nu;
	}

	if env::var_os("PSModulePath").is_some() {
		return ShellKind::PowerShell;
	}

	if cfg!(windows)
		&& let Some(comspec) = env::var_os("ComSpec")
		&& comspec
		.to_string_lossy()
		.to_ascii_lowercase()
		.contains("cmd.exe")
	{
		return ShellKind::Cmd;
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
		let parsed = manage_last_command::parse_shell_name(&process_name);
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
	if name.is_empty() { None } else { Some(name) }
}

/// Detects a shell by traversing parent processes through WMI on Windows.
fn detect_shell_from_parent_process_windows() -> Option<ShellKind> {
	let mut pid = process::id();

	for _ in 0..8 {
		let parent_pid = windows_parent_pid(pid)?;
		let process_name = windows_process_name(parent_pid)?;
		let parsed = manage_last_command::parse_shell_name(&process_name);
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
	let script = format!(
		"(Get-CimInstance Win32_Process -Filter \"ProcessId={}\").Name",
		pid
	);
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

		if let Ok(value) = output
			&& value.status.success()
		{
			let text = String::from_utf8_lossy(&value.stdout).trim().to_owned();
			if !text.is_empty() {
				return Some(text);
			}
		}
	}

	None
}
