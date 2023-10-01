use super::PackageArgs;
use crate::{return_error, Cli, Result, WrapErr};

/// Check a package for errors.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(short, long, default_value = ".")]
	pub package: tg::package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,
}

impl Cli {
	pub async fn command_check(&self, args: Args) -> Result<()> {
		todo!()

		// // Get the package.
		// let package = tg::Package::with_specifier(&self.client, args.package)
		// 	.await
		// 	.wrap_err("Failed to get the package.")?;

		// // Check the package for diagnostics.
		// let diagnostics = package.check(&self.client).await?;

		// // Print the diagnostics.
		// for diagnostic in &diagnostics {
		// 	// Get the diagnostic location and message.
		// 	let tg::language::Diagnostic {
		// 		location, message, ..
		// 	} = diagnostic;

		// 	// Print the location if one is available.
		// 	if let Some(location) = location {
		// 		let tg::language::Location { module, range, .. } = location;
		// 		let tg::language::Position { line, character } = range.start;
		// 		let line = line + 1;
		// 		let character = character + 1;

		// 		println!("{module}:{line}:{character}");
		// 	}

		// 	// Print the diagnostic message.
		// 	println!("{message}");

		// 	// Print a newline.
		// 	println!();
		// }

		// if !diagnostics.is_empty() {
		// 	return_error!("Type checking failed.");
		// }

		// Ok(())
	}
}
