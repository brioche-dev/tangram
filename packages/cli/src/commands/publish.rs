use crate::Cli;
use std::path::PathBuf;
use tangram_client as tg;
use tangram_package::PackageExt;
use tg::{Result, WrapErr};

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
		let client = self.client.as_deref().unwrap();

		// Create the package.
		let package = tg::Package::with_path(client, &args.package).await?;

		// Get the package ID.
		let id = package.id(client).await?;

		// Publish the package.
		client
			.publish_package(id)
			.await
			.wrap_err("Failed to publish the package.")?;

		Ok(())
	}
}
