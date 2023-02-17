use crate::{
	operation::{Download, Operation},
	Cli,
};
use anyhow::{Context, Result};
use std::sync::Arc;
use url::Url;

/// Run a download operation.
#[derive(clap::Args)]
pub struct Args {
	/// The URL to download from.
	url: Url,

	unpack: bool,
}

impl Cli {
	pub async fn command_download(self: &Arc<Self>, args: Args) -> Result<()> {
		// Create the operation.
		let operation = Operation::Download(Download {
			url: args.url,
			unpack: args.unpack,
			checksum: None,
			is_unsafe: true,
		});

		// Run the operation.
		let output = self
			.run(&operation)
			.await
			.context("Failed to run the operation.")?;

		// Print the output.
		println!("{output:?}");

		Ok(())
	}
}
