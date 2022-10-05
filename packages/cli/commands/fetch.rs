use crate::Cli;
use anyhow::{Context, Result};
use clap::Parser;
use url::Url;

#[derive(Parser)]
pub struct Args {
	#[arg(help = "The URL to fetch.")]
	url: Url,
	#[arg(long, help = "If the URL points to a tarball, should it be unpacked?")]
	unpack: bool,
}

impl Cli {
	pub(crate) async fn command_fetch(&self, args: Args) -> Result<()> {
		// Lock the builder.
		let builder = self.builder.lock_shared().await?;

		// Create the expression.
		let hash = builder
			.add_expression(&tangram_core::expression::Expression::Fetch(
				tangram_core::expression::Fetch {
					url: args.url,
					unpack: args.unpack,
					hash: None,
				},
			))
			.await?;

		// Evaluate the expression.
		let output_hash = builder
			.evaluate(hash, hash)
			.await
			.context("Failed to evaluate the expression.")?;

		// Print the output.
		let output = builder.get_expression(output_hash).await?;
		let output_json =
			serde_json::to_string_pretty(&output).context("Failed to serialize the value.")?;
		println!("{output_json}");

		Ok(())
	}
}
