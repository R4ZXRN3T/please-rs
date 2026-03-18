mod manage_last_command;

use anyhow::{anyhow, Context};
use std::env::args;
use std::process::{self, Command, Stdio};

fn execute_command(command_args: &[String]) -> anyhow::Result<i32> {
	if command_args.is_empty() {
		return Err(anyhow!("illegal command length of zero"));
	}

	let mut command = Command::new(&command_args[0]);
	command
		.args(&command_args[1..])
		.stdin(Stdio::inherit())
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit());

	let status = command
		.status()
		.with_context(|| format!("failed to start command '{}'", command_args[0]))?;

	Ok(status.code().unwrap_or(1))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let args: Vec<String> = args().collect();

	let mut command: Vec<String> = if args.len() == 1 {
		manage_last_command::get_last_command()?
	} else {
		args[1..].to_vec()
	};

	command.insert(0, "sudo".to_owned());

	let exit_code = execute_command(&command)?;
	process::exit(exit_code);
}
