use super::PackageArgs;
use crate::{Cli, Result, WrapErr};
use tangram_client as tg;

/// Print the docs for a package.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(short, long, default_value = ".")]
	pub package: tg::package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,
}

impl Cli {
	pub async fn command_doc(&self, args: Args) -> Result<()> {
		// Get the package.
		let package = tg::Package::with_specifier(self.client.as_ref(), args.package)
			.await
			.wrap_err("Failed to get the package.")?;

		// Create the language server.
		let server =
			tangram_lsp::Server::new(self.client.downgrade(), tokio::runtime::Handle::current());

		// Get the docs.
		let docs = server
			.docs(&package.root_module(self.client.as_ref()).await?)
			.await?;

		// Render the docs to JSON.
		let json = serde_json::to_string_pretty(&docs)?;

		// Print the docs.
		println!("{json}");

		Ok(())
	}
}
