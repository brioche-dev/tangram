use crate::Cli;
use anyhow::Result;
use clap::Parser;
use tangram_core::{js, specifier::Specifier};

#[derive(Parser)]
pub struct Args {
	#[arg(long)]
	locked: bool,

	#[arg(default_value = ".")]
	specifier: Specifier,
}

impl Cli {
	pub(crate) async fn command_check(&self, args: Args) -> Result<()> {
		let compiler = js::Compiler::new(self.builder.clone());

		// Check in the package, get its hash.
		let package_hash = self
			.package_hash_for_specifier(&args.specifier, args.locked)
			.await?;

		// Check the package for diagnostics.
		let diagnostics = compiler.check(package_hash).await?;

		// Print the diagnostics.
		for diagnostic in &diagnostics {
			match diagnostic {
				js::Diagnostic::File(js::FileDiagnostic {
					file_name,
					line,
					col,
					message,
				}) => println!("{file_name}:{line}:{col}\n\t{message}\n"),
				js::Diagnostic::Other(js::OtherDiagnostic { message }) => {
					println!("{message}");
				},
			}
		}

		Ok(())
	}
}
