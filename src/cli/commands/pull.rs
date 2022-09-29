use anyhow::{Context, Result};
use clap::Parser;
use tangram::{client::Client, hash::Hash};
use url::Url;

#[derive(Parser, Debug)]
#[command(trailing_var_arg = true)]
pub struct Args {
	#[arg(long)]
	pub hash: Hash,
	#[arg(long)]
	pub url: Url,
}

pub async fn run(args: Args) -> Result<()> {
	// Create the builder.
	let builder = crate::builder().await?.lock_shared().await?;

	// Create the client.
	let client = Client::new(args.url, None);

	builder
		.pull(args.hash, &client)
		.await
		.context("Failed to pull hash from url.")?;

	Ok(())
}
