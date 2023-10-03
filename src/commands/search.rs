use crate::{Cli, Result};

/// Search for packages.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	pub query: String,
}

impl Cli {
	pub async fn command_search(&self, args: Args) -> Result<()> {
		// Perform the search.
		let packages = self.api_client.search_packages(&args.query).await?;

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
