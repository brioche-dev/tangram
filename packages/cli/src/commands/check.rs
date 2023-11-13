use super::PackageArgs;
use crate::Cli;
use tangram_error::{return_error, Result, WrapErr};
use tangram_package::Specifier;

/// Check a package for errors.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(short, long, default_value = ".")]
	pub package: tangram_package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,
}

impl Cli {
	pub async fn command_check(&self, args: Args) -> Result<()> {
		let client = self.client().await?;
		let client = client.as_ref();

		// Get the package.
		let (package, lock) = tangram_package::new(client, &args.package)
			.await
			.wrap_err("Failed to get the package.")?;

		let package_builder: Option<Box<dyn tangram_client::package::Builder>> = if let Specifier::Path(package) = &args.package {
			let builder = tangram_package::Builder::new(package);
			Some(Box::new(builder))
		} else {
			None
		};

		// Create the language server.
		let server = tangram_lsp::Server::new(client, tokio::runtime::Handle::current(), package_builder);

		// Check the package for diagnostics.
		let diagnostics = server
			.check(vec![tangram_lsp::Module::Normal(
				tangram_lsp::module::Normal {
					package: package.id(client).await?,
					lock: lock.id(client).await?.clone(),
					path: tangram_package::ROOT_MODULE_FILE_NAME.parse().unwrap(),
				},
			)])
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
