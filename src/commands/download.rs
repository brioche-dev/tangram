use crate::{
	expression::{self, Expression},
	Cli,
};
use anyhow::{Context, Result};
use clap::Parser;
use url::Url;

#[derive(Parser)]
#[command(long_about = "Evaluate a download expression.")]
pub struct Args {
	#[arg(help = "The URL to download from.")]
	url: Url,
	#[arg(long, help = "If the URL points to a tarball, should it be unpacked?")]
	unpack: bool,
}

impl Cli {
	pub(crate) async fn command_download(&self, args: Args) -> Result<()> {
		// Lock the cli.
		let cli = self.lock_shared().await?;

		// Create the expression.
		let hash = cli
			.add_expression(&Expression::Download(expression::Download {
				url: args.url,
				unpack: args.unpack,
				checksum: None,
			}))
			.await?;

		// Evaluate the expression.
		let output_hash = cli
			.evaluate(hash, hash)
			.await
			.context("Failed to evaluate the expression.")?;

		// Print the output.
		println!("{output_hash}");

		Ok(())
	}
}
