use super::PackageArgs;
use crate::Cli;
use tangram_client as tg;
use tangram_package::PackageExt;
use tg::{return_error, Result, WrapErr};

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
		let client = self.client().await?;
		let client = client.as_ref();

		// Get the package.
		let package = tg::Package::with_specifier(client, args.package.clone())
			.await
			.wrap_err("Failed to get the package.")?;

		// Create the language server.
		let server =
			tangram_lsp::Server::new(client.downgrade_box(), tokio::runtime::Handle::current());

		// Check the package for diagnostics.
		let diagnostics = server
			.check(vec![package.root_module(client).await?])
			.await?;

		// Print the diagnostics.
		for diagnostic in &diagnostics {
			// Get the diagnostic location and message.
			let tangram_lsp::Diagnostic {
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
