use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
pub struct Args {
	#[clap(
		help = "The path to the file or directory to hash. If not provided, stdin will be used."
	)]
	input: Option<String>,
}

pub async fn run(args: Args) -> Result<()> {
	let hash = match args.input {
		Some(path) => {
			let hash = tangram_archive::hash(path).await?;
			hash
		},
		None => {
			let stdin = tokio::io::stdin();
			let (_, hash) = tangram_hash::len_and_hash_for_reader(stdin).await?;
			hash
		},
	};
	println!("{hash}");
	Ok(())
}
