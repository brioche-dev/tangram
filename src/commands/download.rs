use crate::Cli;
use anyhow::{Context, Result};
use tangram::operation::{Download, Operation};
use url::Url;

/// Run a download operation.
#[derive(clap::Args)]
pub struct Args {
	/// The URL to download from.
	url: Url,

	unpack: bool,
}

impl Cli {
	pub async fn command_download(&self, args: Args) -> Result<()> {
		// Create the operation.
		let operation = Operation::Download(Download {
			url: args.url,
			unpack: args.unpack,
			checksum: None,
			is_unsafe: true,
		});
		let operation_hash = self.tg.add_operation(&operation)?;

		// Run the operation.
		let output = self
			.tg
			.run(operation_hash)
			.await
			.context("Failed to run the operation.")?;

		// Print the output.
		println!("{output:?}");

		Ok(())
	}
}
