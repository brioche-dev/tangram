use crate::{
	compiler::{Compiler, Diagnostic, Location, Position},
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
	pub async fn command_check(&self, args: Args) -> Result<()> {
		// If the specifier is a path specifier, first generate its lockfile.
		if let Specifier::Path { path } = &args.specifier {
			self.generate_lockfile(path, args.locked).await?;
		}

		// Create a compiler.
		let compiler = Compiler::new(self.clone());

		// Get the entrypoint module identifier.
		let module_identifier = self
			.entrypoint_module_identifier_for_specifier(&args.specifier)
			.await?;

		// Check the package for diagnostics.
		let diagnostics = compiler.check(vec![module_identifier]).await?;

		// Print the diagnostics.
		for diagnostics in diagnostics.values() {
			for diagnostic in diagnostics {
				// Retrieve the diagnostic location and message.
				let Diagnostic {
					location, message, ..
				} = diagnostic;

				// Print the location if one is available.
				if let Some(location) = location {
					let Location {
						module_identifier: url,
						range,
						..
					} = location;
					let Position { line, character } = range.start;
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
