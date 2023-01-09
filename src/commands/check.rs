use crate::{
	compiler::{self, Compiler},
	specifier::Specifier,
	Cli,
};
use anyhow::{bail, Result};
use clap::Parser;

#[derive(Parser)]
#[command(about = "Check a package for errors.")]
pub struct Args {
	#[arg(long)]
	locked: bool,

	#[arg(default_value = ".")]
	specifier: Specifier,
}

impl Cli {
	pub(crate) async fn command_check(&self, args: Args) -> Result<()> {
		// Lock the cli.
		let cli = self.lock_shared().await?;

		// If the specifier is a path specifier, first generate its lockfile.
		if let Specifier::Path(path) = &args.specifier {
			cli.generate_lockfile(path, args.locked).await?;
		}

		// Create a compiler.
		let compiler = Compiler::new(self.clone());

		// Get the js URLs for the package.
		let urls = cli.js_urls_for_specifier(&args.specifier).await?;

		// Check the package for diagnostics.
		let diagnostics = compiler.check(urls).await?;

		// Print the diagnostics.
		for diagnostics in diagnostics.values() {
			for diagnostic in diagnostics {
				// Retrieve the diagnostic location and message.
				let compiler::types::Diagnostic {
					location, message, ..
				} = diagnostic;

				// Print the location if one is available.
				if let Some(location) = location {
					let compiler::types::Location { url, range, .. } = location;
					let compiler::types::Position { line, character } = range.start;
					let line = line + 1;
					let character = character + 1;

					println!("{url}:{line}:{character}");
				}

				// Print the diagnostic message.
				println!("{message}");

				// Print a newline.
				println!();
			}
		}

		if !diagnostics.is_empty() {
			bail!("Type checking failed.");
		}

		Ok(())
	}
}
