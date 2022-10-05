use crate::Cli;
use anyhow::{Context, Result};
use clap::Parser;
use futures::TryStreamExt;
use tokio::io::AsyncRead;
use url::Url;

#[derive(Parser)]
pub struct Args {
	input: Option<String>,
}

impl Cli {
	pub(crate) async fn command_hash(&self, args: Args) -> Result<()> {
		// Create the hasher.
		let mut hasher = tangram_core::hash::Hasher::new();

		// Get the input.
		let mut input: Box<dyn AsyncRead + Unpin + Send + Sync> = if let Some(input) = args.input {
			if let Ok(input) = Url::parse(&input) {
				// Send the request.
				let response = reqwest::get(input).await?.error_for_status()?;

				// Create a reader from the stream.
				let response = response
					.bytes_stream()
					.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error));
				let response = tokio_util::io::StreamReader::new(response);
				Box::new(response)
			} else {
				let file = tokio::fs::File::open(&input)
					.await
					.with_context(|| format!(r#"Failed to open the file at path "{input}"."#))?;
				Box::new(file)
			}
		} else {
			Box::new(tokio::io::stdin())
		};

		// Copy the input into the hasher.
		tokio::io::copy(&mut input, &mut hasher).await?;

		// Finalize the hash.
		let hash = hasher.finalize();

		// Print the hash.
		println!("{hash}");

		Ok(())
	}
}
