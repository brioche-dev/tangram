use crate::credentials::Credentials;
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use url::Url;

#[derive(Parser)]
pub struct Args {
	#[clap(
		long,
		help = "The URI of the API to publish to. Defaults to https://api.tangram.dev.",
		default_value = "https://api.tangram.dev"
	)]
	url: Url,
	#[clap(long, default_value = ".")]
	package: PathBuf,
}

pub async fn run(args: Args) -> Result<()> {
	// Create the API client.
	let _client = tangram_api_client::Transport::new(&args.uri);

	// Retrieve the credentials for the specified API.
	let credentials = Credentials::read().await?;
	let _credentials_entry = credentials.get(&args.uri).unwrap();

	// Publish!
	// client
	// 	.publish_package(args.package, credentials_entry.token)
	// 	.await?;

	Ok(())
}
