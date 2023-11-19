use crate::Cli;
use std::path::PathBuf;
use tangram_error::{Result, WrapErr};

/// Publish a package.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(short, long, default_value = ".")]
	pub package: PathBuf,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_publish(&self, args: Args) -> Result<()> {
		let client = self.client().await?;
		let client = client.as_ref();
		let user = self.user().await?;

		// Create the package.
		let specifier = tangram_lsp::package::Specifier::Path(args.package);
		let lsp = tangram_lsp::Server::new(client, tokio::runtime::Handle::current());
		let (package, _) = lsp.create_package(&specifier).await?;

		// Get the package ID.
		let id = package.id(client).await?;

		// Publish the package.
		client
			.publish_package(user.as_ref(), &id)
			.await
			.wrap_err("Failed to publish the package.")?;

		Ok(())
	}
}
