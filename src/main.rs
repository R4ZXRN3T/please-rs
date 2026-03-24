mod get_current_shell;
mod manage_last_command;

use std::env::{self, args};
use std::error::Error;
use std::io;
use std::process::{self, Command, Stdio};

/// Executes a system command with the given arguments.
///
/// # Arguments
///
/// * `command_args` - A slice of strings where the first element is the command name
///   and the remaining elements are arguments to pass to the command.
///
/// # Returns
///
/// Returns the child process exit code, or `1` when no platform-specific code is available.
///
/// # Errors
///
/// Returns an error if `command_args` is empty or the child process fails to start.
fn execute_command(command_args: &[String]) -> Result<i32, Box<dyn Error>> {
	if command_args.is_empty() {
		return Err(io::Error::other("illegal command length of zero").into());
	}

	let mut command = Command::new(&command_args[0]);
	command
		.args(&command_args[1..])
		.stdin(Stdio::inherit())
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit());

	let status = command.status().map_err(|err| {
		io::Error::other(format!(
			"failed to start command '{}': {err}",
			command_args[0]
		))
	})?;

	Ok(status.code().unwrap_or(1))
}

/// Prints the help message to stdout.
///
/// Displays usage information, available options, and examples for the `please` command.
fn print_help() {
	println!(
		"please - Run commands with sudo or replay your last command.\n\
\n\
USAGE:\n\
  please [COMMAND] [ARG]...\n\
  please [OPTIONS]\n\
\n\
OPTIONS:\n\
  -h, --help         Show this message\n\
  -p, --print-shell  Print the detected shell\n\
  -i, --info         Print detailed runtime information\n\
  -v, --version      Print version\n\
\n\
BEHAVIOR:\n\
  With arguments: runs `sudo <arguments...>`.\n\
  Without arguments: finds the last non-`please` command from history and runs it with sudo.\n\
  Override shell detection with the `PLEASE_SHELL` environment variable.\n\
\n\
EXAMPLES:\n\
  please apt update\n\
  please systemctl restart nginx\n\
	  please\n\
  please --print-shell"
	)
}

/// Prints only the application version.
fn print_version() {
	println!("{}", env!("CARGO_PKG_VERSION"));
}

/// Prints detailed runtime information.
fn print_info() {
	let detected_shell = get_current_shell::detect_shell();
	let forced_shell = env::var("PLEASE_SHELL").ok();
	let forced_shell = forced_shell.as_deref().unwrap_or("<not set>");

	println!(
		"{} v{}\n\
Target: {}-{}\n\
Detected shell: {}\n\
PLEASE_SHELL: {}\n\
\n\
Run commands with sudo or replay your last non-`please` command.\n\
For usage details, run `please --help`.",
		env!("CARGO_PKG_NAME"),
		env!("CARGO_PKG_VERSION"),
		env::consts::OS,
		env::consts::ARCH,
		detected_shell,
		forced_shell
	);
}

/// Entry point for the `please` command-line application.
///
/// Processes command-line arguments, handles built-in options, and then either:
/// - runs a provided command through `sudo`, or
/// - retrieves the last non-`please` command from shell history and runs it through `sudo`.
///
/// # Returns
///
/// Returns `Ok(())` for built-in informational options.
/// For command execution paths, this function exits the process with the child exit code.
///
/// # Errors
///
/// Returns an error if history discovery fails (no-argument mode) or if spawning a command fails.
fn main() -> Result<(), Box<dyn Error>> {
	let args: Vec<String> = args().collect();
	if args.len() == 2 {
		match args[1].as_str() {
			"-h" | "--help" => {
				print_help();
				return Ok(());
			}
			"-v" | "--version" => {
				print_version();
				return Ok(());
			}
			"-p" | "--print-shell" => {
				println!("Detected shell: {}", get_current_shell::detect_shell());
				return Ok(());
			}
			"-i" | "--info" => {
				print_info();
				return Ok(());
			}
			_ => (),
		}
	}

	let mut command: Vec<String> = match args.len() {
		1 => manage_last_command::get_last_command()?,
		_ => args[1..].to_vec(),
	};

	command.insert(0, "sudo".to_owned());

	let exit_code = execute_command(&command)?;
	process::exit(exit_code);
}
