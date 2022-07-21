use crate::client::create;
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
pub struct Args {
	#[clap(
		help = "Source the file or directory at this path. If not provided, stdin will be used instead."
	)]
	input: Option<String>,
	#[clap(
		long,
		help = "If we're sourcing a directory, should it be processed as a Tangram package?"
	)]
	package: bool,
	#[clap(long, help = "If we're sourcing a tarball, should it be unpacked?")]
	unpack: bool,
}

pub async fn run(args: Args) -> Result<()> {
	let client = create().await?;
	let artifact = match args.input {
		Some(path) => {
			client
				.source_path_artifact(path, args.package, args.unpack)
				.await?
		},
		None => {
			let mut stdin = tokio::io::stdin();
			client.source_reader(&mut stdin, None).await?
		},
	};
	let hash = artifact.hash;
	println!("{hash}");
	Ok(())
}
