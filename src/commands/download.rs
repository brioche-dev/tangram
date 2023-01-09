use crate::{
	operation::{Download, Operation},
	Cli,
};
use anyhow::{Context, Result};
use clap::Parser;
use url::Url;

#[derive(Parser)]
#[command(about = "Run a download operation.")]
pub struct Args {
	#[arg(help = "The URL to download from.")]
	url: Url,
	#[arg(long, help = "If the URL points to a tarball, should it be unpacked?")]
	unpack: bool,
}

impl Cli {
	pub(crate) async fn command_download(&self, args: Args) -> Result<()> {
		// Lock the cli.
		let cli = self
			.lock_shared()
			.await
			.context("Failed to lock the cli.")?;

		// Create the operation.
		let operation = Operation::Download(Download {
			url: args.url,
			unpack: args.unpack,
			checksum: None,
			is_unsafe: true,
		});

		// Run the operation.
		let output = cli
			.run(&operation)
			.await
			.context("Failed to run the operation.")?;

		// Print the output.
		println!("{output:?}");

		Ok(())
	}
}
