use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
pub struct Args {}

pub async fn run(_args: Args) -> Result<()> {
	let client = crate::client::new().await?;
	let repl_id = client.create_repl().await?;
	let mut readline = rustyline::Editor::<()>::new()?;

	// This is the REPL loop.
	loop {
		// Read a line of code.
		let code = match readline.readline("> ") {
			Ok(code) => code,
			Err(rustyline::error::ReadlineError::Interrupted) => {
				continue;
			},
			Err(rustyline::error::ReadlineError::Eof) => {
				break;
			},
			Err(error) => {
				println!("{:?}", error);
				continue;
			},
		};

		// Run the code.
		let output = client.repl_run(repl_id, &code).await?;
		match output {
			Ok(Some(output)) => println!("{}", output),
			Ok(None) => {},
			Err(message) => println!("{}", message),
		}
	}

	Ok(())
}
