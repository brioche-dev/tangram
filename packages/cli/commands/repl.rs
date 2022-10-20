use crate::Cli;
use anyhow::Result;
use clap::Parser;
use tangram_core::js;

#[derive(Parser)]
#[command(long_about = "Read, Eval, Print, Loop.")]
pub struct Args {}

impl Cli {
	pub async fn command_repl(&self, _args: Args) -> Result<()> {
		// Lock the builder.
		let builder = self.builder.lock_shared().await?;
		let main_runtime_handle = tokio::runtime::Handle::current();

		std::thread::spawn(|| {
			// Create a single threaded tokio runtime.
			let rt = tokio::runtime::Builder::new_current_thread()
				.enable_all()
				.build()
				.unwrap();

			// Run the REPL.
			rt.block_on(async move {
				// Create the runtime.
				let mut runtime = js::Runtime::new(builder, main_runtime_handle)
					.await
					.unwrap();

				// This is the REPL loop.
				let mut readline = rustyline::Editor::<()>::new().unwrap();
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
			});
		})
		.join()
		.unwrap();

		Ok(())
	}
}
