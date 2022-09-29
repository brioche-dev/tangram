use crate::client::new;
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
pub struct Args {
	name: String,
}

pub async fn run(args: Args) -> Result<()> {
	// Create the client.
	let client = new().await?;

	// Search for the package with the given name.
	let package_name = args.name;
	let packages = client.search(&package_name).await?;
	println!("{:?}", packages);

	Ok(())
}
