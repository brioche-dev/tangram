use crate::config::Config;
use anyhow::{Context, Result};
use clap::Parser;
use tangram::client::Client;

#[derive(Parser)]
pub struct Args {}

pub async fn run(_args: Args) -> Result<()> {
	// Read the config.
	let config = Config::read().await.context("Failed to read the config.")?;

	// Create the client.
	let client = Client::new_with_config(config.client)
		.await
		.context("Failed to create the client.")?;

	// Create the REPL.
	let repl_id = client.create_repl().await?;

	// This is the REPL loop.
	let mut readline = rustyline::Editor::<()>::new()?;
	loop {
		// R: Read a line of code.
		let code = match readline.readline("> ") {
			Ok(code) => code,
			Err(rustyline::error::ReadlineError::Interrupted) => {
				continue;
			},
			Err(rustyline::error::ReadlineError::Eof) => {
				break;
			},
			Err(error) => {
				println!("{error:?}");
				continue;
			},
		};

		// E: Evaluate the code.
		let output = client.repl_run(repl_id, code).await?;

		// P: Print the output.
		match output {
			tangram::server::repl::Output::Success {
				message: Some(output),
			} => {
				println!("{}", output);
			},

			tangram::server::repl::Output::Success { message: None } => {},

			tangram::server::repl::Output::Error { message } => {
				println!("{}", message);
			},
		}

		// L: Loop!
	}

	Ok(())
}
