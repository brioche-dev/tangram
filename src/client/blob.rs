use super::Client;
use crate::{
	blob,
	error::{Context, Result},
};
use futures::TryStreamExt;
use std::{pin::Pin, sync::Arc};
use tokio::io::AsyncRead;
use tokio_util::io::StreamReader;

impl Client {
	pub async fn add_blob<R>(&self, reader: R, blob_hash: blob::Hash) -> Result<blob::Hash>
	where
		R: AsyncRead + Send + Sync + Unpin + 'static,
	{
		// Get a permit.
		let _permit = self.socket_semaphore.acquire().await?;

		// Create a stream for the body.
		let stream = tokio_util::io::ReaderStream::new(reader);
		let body = hyper::Body::wrap_stream(stream);

		// Build the URL.
		let path = format!("/v1/blobs/{blob_hash}");
		let mut url = self.url.clone();
		url.set_path(&path);

		// Send the request.
		let response = self
			.request(http::Method::POST, url)
			.body(body)
			.send()
			.await?
			.error_for_status()?;

		// Read the response.
		let response = response.bytes().await?;
		let blob_hash = String::from_utf8(response.to_vec())
			.context("Failed to read the response as UTF-8.")?
			.parse()
			.context("Failed to parse the blob hash.")?;

		Ok(blob_hash)
	}
}

impl Client {
	pub async fn get_blob(&self, blob_hash: blob::Hash) -> Result<impl AsyncRead> {
		// Get a permit.
		let permit = Arc::clone(&self.socket_semaphore).acquire_owned().await?;

		// Build the URL.
		let path = format!("/v1/blobs/{blob_hash}");
		let mut url = self.url.clone();
		url.set_path(&path);

		// Send the request.
		let response = self
			.request(http::Method::GET, url)
			.send()
			.await?
			.error_for_status()?;

		// Read the response body.
		let body = response
			.bytes_stream()
			.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error));

		// Create an async reader from the body.
		let body = StreamReader::new(body);

		Ok(AsyncReaderWithPermit {
			reader: body,
			permit,
		})
	}
}

pub struct AsyncReaderWithPermit<R>
where
	R: AsyncRead,
{
	pub reader: R,
	pub permit: tokio::sync::OwnedSemaphorePermit,
}

impl<R> AsyncRead for AsyncReaderWithPermit<R>
where
	R: AsyncRead + Unpin,
{
	fn poll_read(
		mut self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		buf: &mut tokio::io::ReadBuf<'_>,
	) -> std::task::Poll<std::io::Result<()>> {
		Pin::new(&mut self.reader).poll_read(cx, buf)
	}
}
