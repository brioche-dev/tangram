use crate::Cli;
use anyhow::Result;
use clap::Parser;
use tangram_core::{builder::Builder, js};

#[derive(Parser)]
#[command(long_about = "Read, Eval, Print, Loop.")]
pub struct Args {}

impl Cli {
	pub async fn command_repl(&self, _args: Args) -> Result<()> {
		// Get a handle to the current tokio runtime.
		let main_runtime_handle = tokio::runtime::Handle::current();

		// Spawn a thread to run the REPL loop.
		std::thread::spawn({
			let builder = self.builder.clone();
			move || {
				// Create a single threaded tokio runtime.
				let runtime = tokio::runtime::Builder::new_current_thread()
					.enable_all()
					.build()
					.unwrap();

				// Run the REPL.
				runtime.block_on(async move {
					repl(builder, main_runtime_handle).await.unwrap();
				});
			}
		})
		.join()
		.unwrap();

		Ok(())
	}
}

async fn repl(builder: Builder, main_runtime_handle: tokio::runtime::Handle) -> Result<()> {
	// Create the runtime.
	let mut runtime = js::Runtime::new(builder, main_runtime_handle).await?;

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
		let result = runtime.repl(&code).await;

		// P: Print the output.
		match result {
			Ok(Some(output)) => {
				println!("{}", output);
			},

			Ok(None) => {},

			Err(message) => {
				println!("{}", message);
			},
		}

		// L: Loop!
	}

	Ok(())
}
