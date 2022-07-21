use anyhow::Result;
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
	let Args { url, unpack } = args;
	let expression = tangram::expression::Expression::Fetch(tangram::expression::Fetch {
		url,
		unpack,
		hash: None,
	});
	let client = crate::client::new().await?;
	let value = client.evaluate(expression).await?;
	let value = serde_json::to_string_pretty(&value)?;
	println!("{value}");
	Ok(())
}
