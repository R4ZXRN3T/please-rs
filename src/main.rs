mod manage_last_command;

use std::env::args;
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
/// Returns the exit code of the executed command, or an error if the command failed to start.
///
/// # Errors
///
/// Returns an error if the command arguments are empty or if the command fails to start.
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
/// Displays usage information, behavior description, and examples for the `please` command.
fn print_help() {
  println!(
	"please - run commands with sudo, or re-run the last saved command.\n\
\n\
USAGE:\n\
  please [COMMAND] [ARG]...\n\
  please [OPTIONS]\n\
\n\
OPTIONS:\n\
  -h, --help         Show this message\n\
  -p, --print-shell  Print the detected shell\n\
  -i, --info         Print version and basic information\n\
\n\
BEHAVIOR:\n\
  With arguments: runs `sudo <arguments...>`.\n\
  Without arguments: loads the last saved command and runs it with sudo.\n\
\n\
EXAMPLES:\n\
  please apt update\n\
  please systemctl restart nginx\n\
  please"
  )
}

fn print_info() {
	println!(
		"{} v{}\n\nRun commands with sudo, or re-run the last saved command.\n\nFor usage details, run `please --help`.",
		env!("CARGO_PKG_NAME"),
		env!("CARGO_PKG_VERSION")
	);
}

/// Entry point for the `please` command-line application.
///
/// Processes command-line arguments and either runs a given command with `sudo` or retrieves
/// and runs the last saved command from shell history.
///
/// # Returns
///
/// Returns `Ok(())` on success or an error if command execution fails.
///
/// # Errors
///
/// Returns an error if the command fails to execute or if the shell detection fails.
fn main() -> Result<(), Box<dyn Error>> {
	let args: Vec<String> = args().collect();
	if args.len() == 2 {
		match args[1].as_str() {
			"-h" | "--help" => {
				print_help();
				return Ok(());
			}
			"-p" | "--print-shell" => {
				println!("Detected shell: {}", manage_last_command::detect_shell());
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
