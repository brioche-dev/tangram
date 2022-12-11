use crate::Cli;
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
pub struct Args {
	query: String,
}

impl Cli {
	pub(crate) async fn command_search(&self, args: Args) -> Result<()> {
		// Search for the package with the given query.
		let packages = self
			.lock_shared()
			.await?
			.api_client
			.search_packages(&args.query)
			.await?;

		// Print the package names.
		if packages.is_empty() {
			println!("No packages matched your query.");
		}
		for package in packages {
			let name = package.name;
			println!("{name}");
		}

		Ok(())
	}
}
