use super::PackageArgs;
use crate::{
	error::{return_error, Result, WrapErr},
	Cli,
};
use tangram::{
	language::{location::Location, Diagnostic},
	module::position::Position,
	module::Module,
	package::{self, Package},
};

/// Check a package for errors.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(short, long, default_value = ".")]
	pub package: package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,
}

impl Cli {
	pub async fn command_check(&self, args: Args) -> Result<()> {
		// Get the package.
		let package = Package::with_specifier(&self.tg, args.package)
			.await
			.wrap_err("Failed to get the package.")?;

		// Create the package instance.
		let package_instance = package
			.instantiate(&self.tg)
			.await
			.wrap_err("Failed to create the package instance.")?;

		// Get the root module.
		let root_module = package_instance.root_module();

		// Check the package for diagnostics.
		let diagnostics = Module::check(&self.tg, vec![root_module]).await?;

		// Print the diagnostics.
		for diagnostic in &diagnostics {
			// Get the diagnostic location and message.
			let Diagnostic {
				location, message, ..
			} = diagnostic;

			// Print the location if one is available.
			if let Some(location) = location {
				let Location { module, range, .. } = location;
				let Position { line, character } = range.start;
				let line = line + 1;
				let character = character + 1;

				println!("{module}:{line}:{character}");
			}

			// Print the diagnostic message.
			println!("{message}");

			// Print a newline.
			println!();
		}

		if !diagnostics.is_empty() {
			return_error!("Type checking failed.");
		}

		Ok(())
	}
}
