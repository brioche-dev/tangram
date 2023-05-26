use crate::{error::Result, Cli};
use std::path::PathBuf;

/// Publish a package.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(short, long, default_value = ".")]
	pub package: PathBuf,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_publish(&self, _args: Args) -> Result<()> {
		unimplemented!()

		// // Check in the package.
		// let package = Package::check_in(&self.tg, &args.package).await?;

		// // Create a client.
		// let client = Client::new(
		// 	self.tg.api_client().url.clone(),
		// 	self.tg.api_client().token.clone(),
		// );

		// // Push the package to the registry.
		// let artifact = package.artifact(&self.tg).await?.unwrap();
		// client
		// 	.push(&self.tg, artifact)
		// 	.await
		// 	.wrap_err("Failed to push the package.")?;

		// // Publish the package.
		// self.tg
		// 	.api_client()
		// 	.publish_package()
		// 	.await
		// 	.wrap_err("Failed to publish the package.")?;

		// Ok(())
	}
}
