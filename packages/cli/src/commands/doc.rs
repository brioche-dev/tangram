use super::PackageArgs;
use crate::Cli;
use tangram_client as tg;
use tangram_package::PackageExt;
use tg::{Result, WrapErr};

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
		let client = self.client().await?;
		let client = client.as_ref();

		// Get the package.
		let package = tg::Package::with_specifier(client, args.package)
			.await
			.wrap_err("Failed to get the package.")?;

		// Create the language server.
		let server =
			tangram_lsp::Server::new(client.downgrade_box(), tokio::runtime::Handle::current());

		// Get the docs.
		let docs = server.docs(&package.root_module(client).await?).await?;

		// Render the docs to JSON.
		let json =
			serde_json::to_string_pretty(&docs).wrap_err("Failed to serialize to the docs.")?;

		// Print the docs.
		println!("{json}");

		Ok(())
	}
}
