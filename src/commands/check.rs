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
		// Get the package.
		let package = tg::Package::with_specifier(self.client.as_ref(), args.package)
			.await
			.wrap_err("Failed to get the package.")?;

		// Create the language server.
		let server = tg::lsp::Server::new(self.client.as_ref());

		// Check the package for diagnostics.
		let diagnostics = server
			.check(vec![package.root_module(self.client.as_ref()).await?])
			.await?;

		// Print the diagnostics.
		for diagnostic in &diagnostics {
			// Get the diagnostic location and message.
			let tg::module::Diagnostic {
				location, message, ..
			} = diagnostic;

			// Print the location if one is available.
			if let Some(location) = location {
				println!("{location}");
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
