use crate::{
	error::{Result, WrapErr},
	Cli,
};
use std::path::PathBuf;
use tangram::package::Package;

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
		let package = Package::with_path(&self.tg, &args.package).await?;

		// Push the package.
		self.tg
			.api_client()
			.push(&self.tg, package.artifact())
			.await
			.wrap_err("Failed to push the package.")?;

		// Publish the package.
		self.tg
			.api_client()
			.publish_package(package.hash())
			.await
			.wrap_err("Failed to publish the package.")?;

		Ok(())
	}
}
