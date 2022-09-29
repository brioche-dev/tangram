use anyhow::{Context, Result};
use clap::Parser;
use url::Url;

#[derive(Parser)]
pub struct Args {
	#[clap(help = "The URL to fetch.")]
	url: Url,
	#[clap(long, help = "If the URL points to a tarball, should it be unpacked?")]
	unpack: bool,
}

pub async fn run(args: Args) -> Result<()> {
	// Create the client.
	let client = crate::client::new().await?;

	// Create the expression.
	let hash = client
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
		.evaluate(hash)
		.await
		.context("Failed to evaluate the expression.")?;

	// Print the output.
	let output = client.get_expression(output_hash).await?;
	let output_json =
		serde_json::to_string_pretty(&output).context("Failed to serialize the value.")?;
	println!("{output_json}");

	Ok(())
}
