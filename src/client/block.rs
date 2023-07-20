use super::Client;
use crate::{
	error::{Result, WrapErr},
	id::Id,
};
use futures::TryStreamExt;
use tokio::io::AsyncRead;
use tokio_util::io::{ReaderStream, StreamReader};

impl Client {
	pub async fn try_get_block(&self, id: Id) -> Result<Option<impl AsyncRead>> {
		let _permit = self.semaphore.acquire().await.unwrap();

		// Build the URL.
		let path = format!("/v1/blocks/{id}");
		let mut url = self.url.clone();
		url.set_path(&path);

		// Send the request.
		let response = self
			.request(http::Method::GET, url)
			.send()
			.await
			.wrap_err("Failed to send the request.")?;

		// Check if the block exists.
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}

		// Check if the request was successful.
		let response = response.error_for_status()?;

		// Get the reader.
		let reader = StreamReader::new(
			response
				.bytes_stream()
				.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error)),
		);

		Ok(Some(reader))
	}
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum TryAddBlockOutcome {
	Added,
	MissingChildren(Vec<Id>),
}

impl Client {
	pub async fn try_add_block<R>(&self, id: Id, reader: R) -> Result<TryAddBlockOutcome>
	where
		R: AsyncRead + Send + Sync + Unpin + 'static,
	{
		let _permit = self.semaphore.acquire().await.unwrap();

		// Build the URL.
		let path = format!("/v1/blocks/{id}");
		let mut url = self.url.clone();
		url.set_path(&path);

		// Create the body.
		let body = reqwest::Body::wrap_stream(ReaderStream::new(reader));

		// Send the request.
		let outcome = self
			.request(http::Method::PUT, url)
			.body(body)
			.send()
			.await
			.wrap_err("Failed to send the request.")?
			.error_for_status()?
			.json()
			.await?;

		Ok(outcome)
	}
}
