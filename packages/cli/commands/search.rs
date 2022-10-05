use crate::Cli;
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
pub struct Args {
	name: String,
}

impl Cli {
	pub(crate) async fn command_search(&self, args: Args) -> Result<()> {
		// Create the client.
		let builder = crate::builder().await?;

		// Search for the package with the given name.
		let package_name = args.name;
		let packages = builder
			.lock_shared()
			.await?
			.search_packages(&package_name)
			.await?;

		for package in packages {
			let name = package.name;
			println!("{name}");
		}

		Ok(())
	}
}
