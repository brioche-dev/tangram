use anyhow::{bail, Context, Result};
use clap::Parser;
use futures::TryStreamExt;
use tokio::io::AsyncRead;
use url::Url;

#[derive(Parser)]
pub struct Args {
	input: Option<String>,
}

pub async fn run(args: Args) -> Result<()> {
	// Create the hasher.
	let mut hasher = tangram::hash::Hasher::new();

	// Create the HTTP client.
	let client: hyper::Client<_, hyper::Body> = hyper::Client::builder().build(
		hyper_rustls::HttpsConnectorBuilder::new()
			.with_native_roots()
			.https_or_http()
			.enable_http1()
			.build(),
	);

	// Get the input.
	let mut input: Box<dyn AsyncRead + Unpin + Send + Sync> = if let Some(input) = args.input {
		if let Ok(input) = Url::parse(&input) {
			// Perform the request.
			let request = http::Request::builder()
				.uri(input.to_string())
				.body(hyper::Body::empty())
				.unwrap();
			let response = client.request(request).await?;

			// Handle a non-success status.
			if !response.status().is_success() {
				let status = response.status();
				let body = hyper::body::to_bytes(response.into_body())
					.await
					.context("Failed to read the response body.")?;
				let body = String::from_utf8(body.to_vec())
					.context("Failed to read the response body as a string.")?;
				bail!("{status}\n{body}");
			}

			// Create a reader from the stream.
			let response = response
				.into_body()
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
