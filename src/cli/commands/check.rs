use super::PackageArgs;
use crate::{
	error::{return_error, Result},
	Cli,
};
use tangram::{
	language::{Diagnostic, Location, Position},
	module, package,
};

/// Check a package for errors.
#[derive(Debug, clap::Args)]
pub struct Args {
	#[arg(short, long, default_value = ".")]
	pub package: package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,
}

impl Cli {
	pub async fn command_check(&self, args: Args) -> Result<()> {
		// Resolve the package specifier.
		let package_identifier = self.tg.resolve_package(&args.package, None).await?;

		// Create the package instance.
		let package_instance_hash = self
			.tg
			.clone()
			.create_package_instance(&package_identifier, args.package_args.locked)
			.await?;

		// Create the root module identifier.
		let root_module_identifier =
			module::Identifier::for_root_module_in_package_instance(package_instance_hash);

		// Check the package for diagnostics.
		let diagnostics = self.tg.check(vec![root_module_identifier]).await?;

		// Print the diagnostics.
		for diagnostic in &diagnostics {
			// Get the diagnostic location and message.
			let Diagnostic {
				location, message, ..
			} = diagnostic;

			// Print the location if one is available.
			if let Some(location) = location {
				let Location {
					module_identifier,
					range,
					..
				} = location;
				let Position { line, character } = range.start;
				let line = line + 1;
				let character = character + 1;

				println!("{module_identifier}:{line}:{character}");
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
