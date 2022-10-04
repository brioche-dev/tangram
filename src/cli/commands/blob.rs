use anyhow::Result;
use clap::Parser;
use tangram::hash::Hash;

#[derive(Parser)]
pub struct Args {
	#[command(subcommand)]
	subcommand: Subcommand,
}

#[derive(Parser)]
pub enum Subcommand {
	Get(GetArgs),
}

#[derive(Parser, Debug)]
pub struct GetArgs {
	blob_hash: Hash,
}

pub async fn run(args: Args) -> Result<()> {
	match args.subcommand {
		Subcommand::Get(args) => get(args),
	}
	.await?;
	Ok(())
}

pub async fn get(args: GetArgs) -> Result<()> {
	// Create the builder.
	let builder = crate::builder().await?;

	// Get the blob.
	let blob_path = builder
		.lock_shared()
		.await?
		.get_blob(args.blob_hash)
		.await?;

	// Open the blob file.
	let mut file = tokio::fs::File::open(blob_path).await?;

	// Open stdout.
	let mut stdout = tokio::io::stdout();

	// Copy the blob to stdout.
	tokio::io::copy(&mut file, &mut stdout).await?;

	Ok(())
}
