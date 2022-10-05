use crate::Cli;
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
pub struct Args {
	name: String,
}

impl Cli {
	pub(crate) async fn command_search(&self, args: Args) -> Result<()> {
		// Search for the package with the given name.
		let packages = self.api_client.search_packages(&args.name).await?;

		// Print the package names.
		for package in packages {
			let name = package.name;
			println!("{name}");
		}

		Ok(())
	}
}
