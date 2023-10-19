use crate::Cli;
use std::path::PathBuf;
use tangram_client as tg;
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
		// Create the package.
		let package = tg::Package::with_path(self.client.as_ref(), &args.package).await?;

		// Get the package ID.
		let id = package.id(self.client.as_ref()).await?;

		// Publish the package.
		self.client
			.publish_package(id)
			.await
			.wrap_err("Failed to publish the package.")?;

		Ok(())
	}
}
