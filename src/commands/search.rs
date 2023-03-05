use crate::Cli;
use anyhow::Result;

/// Search for a package.
#[derive(clap::Args)]
pub struct Args {
	query: String,
}

impl Cli {
	pub async fn command_search(&self, args: Args) -> Result<()> {
		// Perform the search.
		let packages = self.tg.api_client().search_packages(&args.query).await?;

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
