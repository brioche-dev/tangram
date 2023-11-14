use super::PackageArgs;
use crate::Cli;
use tangram_error::{return_error, Result, WrapErr};
/// Check a package for errors.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(short, long, default_value = ".")]
	pub package: tangram_lsp::package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,
}

impl Cli {
	pub async fn command_check(&self, args: Args) -> Result<()> {
		let client: std::sync::Arc<dyn Client> = self.client().await?;
		let client = client.as_ref();

		// Create the language server.
		let server = tangram_lsp::Server::new(client, tokio::runtime::Handle::current());

		// Get the package.
		let (package, lock) = server.create_package(&args.package).await?;

		// Check the package for diagnostics.
		let diagnostics = server
			.check(vec![tangram_lsp::Module::Normal(
				tangram_lsp::module::Normal {
					package: package.id(client).await?,
					lock: lock.id(client).await?.clone(),
					path: tangram_lsp::package::ROOT_MODULE_FILE_NAME.parse().unwrap(),
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
