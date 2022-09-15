use crate::config::Config;
use anyhow::{Context, Result};
use clap::Parser;
use tangram::client::Client;
use url::Url;

#[derive(Parser)]
pub struct Args {
	#[clap(help = "The URL to fetch.")]
	url: Url,
	#[clap(long, help = "If the URL points to a tarball, should it be unpacked?")]
	unpack: bool,
}

pub async fn run(args: Args) -> Result<()> {
	// Read the config.
	let config = Config::read().await.context("Failed to read the config.")?;

	// Create the client.
	let client = Client::new_with_config(config.client)
		.await
		.context("Failed to create the client.")?;

	// Create the expression.
	let expression = client
		.add_expression(&tangram::expression::Expression::Fetch(
			tangram::expression::Fetch {
				url: args.url,
				unpack: args.unpack,
				hash: None,
			},
		))
		.await?;

	// Evaluate the expression.
	let output_hash = client
		.evaluate(expression)
		.await
		.context("Failed to evaluate the expression.")?;

	// Print the output.
	let output = client.get_expression(output_hash).await?;
	let output_json =
		serde_json::to_string_pretty(&output).context("Failed to serialize the value.")?;
	println!("{output_json}");

	Ok(())
}
