mod manage_last_command;

use std::env::args;
use std::error::Error;
use std::io;
use std::process::{self, Command, Stdio};

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

fn print_help() {
	println!(
		"please - run commands with sudo, or re-run the last saved command.\n\
\n\
Usage:\n\
  please [COMMAND] [ARG]...\n\
  please -h | --help\n\
\n\
Behavior:\n\
  - With arguments: runs `sudo <arguments...>`.\n\
  - Without arguments: loads the last saved command and runs it with sudo.\n\
\n\
Examples:\n\
  please apt update\n\
  please systemctl restart nginx\n\
  please"
	)
}

fn main() -> Result<(), Box<dyn Error>> {
	let args: Vec<String> = args().collect();
	if args.len() == 2 && matches!(args[1].as_str(), "-h" | "--help") {
		print_help();
		return Ok(());
	}

	let mut command: Vec<String> = match args.len() {
		1 => manage_last_command::get_last_command()?,
		_ => args[1..].to_vec(),
	};

	command.insert(0, "sudo".to_owned());

	let exit_code = execute_command(&command)?;
	process::exit(exit_code);
}
