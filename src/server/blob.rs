use super::{error::bad_request, error::not_found, Server};
use crate::{hash::Hasher, object::BlobHash, util::path_exists};
use anyhow::{bail, Context, Result};
use futures::TryStreamExt;
use std::{
	path::{Path, PathBuf},
	sync::Arc,
};
use tokio::io::{AsyncRead, AsyncWriteExt};
use tokio_stream::StreamExt;

impl Server {
	pub async fn add_blob_from_reader(
		self: &Arc<Self>,
		reader: impl AsyncRead + Unpin,
	) -> Result<BlobHash> {
		// Create a temp file to read the blob into.
		let temp = self.create_temp().await?;
		let temp_path = self.temp_path(&temp);
		let mut temp_file = tokio::fs::File::create(&temp_path).await?;

		// Compute the hash of the bytes in the reader and write them to the temp file.
		let mut stream = tokio_util::io::ReaderStream::new(reader);
		let mut hasher = Hasher::new();
		while let Some(chunk) = stream.next().await {
			let chunk = chunk?;
			hasher.update(&chunk);
			temp_file.write_all(&chunk).await?;
		}
		let hash = hasher.finalize();
		let blob_hash = BlobHash(hash);
		temp_file.sync_all().await?;
		drop(temp_file);

		// Move the temp file to the blobs path.
		let blob_path = self.blob_path(blob_hash);
		tokio::fs::rename(&temp_path, &blob_path).await?;

		Ok(blob_hash)
	}

	pub async fn get_blob(self: &Arc<Self>, blob_hash: BlobHash) -> Result<Option<Handle>> {
		let blob_path = self.blob_path(blob_hash);

		// Check if the blob exists.
		if !path_exists(&blob_path).await? {
			return Ok(None);
		}

		Ok(Some(Handle {
			_blob_hash: blob_hash,
			path: blob_path,
		}))
	}

	#[must_use]
	pub fn blobs_path(self: &Arc<Self>) -> PathBuf {
		self.path.join("blobs")
	}

	#[must_use]
	pub fn blob_path(self: &Arc<Self>, blob_hash: BlobHash) -> PathBuf {
		self.path.join("blobs").join(blob_hash.to_string())
	}
}

pub struct Handle {
	_blob_hash: BlobHash,
	path: PathBuf,
}

impl Handle {
	#[must_use]
	pub fn path(&self) -> &Path {
		&self.path
	}
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CreateResponse {
	pub blob_hash: BlobHash,
}

impl Server {
	pub(super) async fn handle_create_blob_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let blob_hash = if let ["blobs", blob_hash] = path_components.as_slice() {
			blob_hash
		} else {
			bail!("Unexpected path.")
		};
		let _blob_hash: BlobHash = match blob_hash.parse() {
			Ok(client_blob_hash) => client_blob_hash,
			Err(_) => return Ok(bad_request()),
		};

		// Create an async reader from the body.
		let body = tokio_util::io::StreamReader::new(
			request
				.into_body()
				.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error)),
		);

		// Add the blob.
		let blob_hash = self.add_blob_from_reader(body).await?;

		// Create the response.
		let response = CreateResponse { blob_hash };
		let response =
			serde_json::to_vec(&response).context("Failed to serialize the response.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::from(response))
			.unwrap();

		Ok(response)
	}

	pub(super) async fn handle_get_blob_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let blob_hash = if let ["blobs", blob_hash] = path_components.as_slice() {
			blob_hash
		} else {
			bail!("Unexpected path.")
		};
		let blob_hash: BlobHash = match blob_hash.parse() {
			Ok(client_blob_hash) => client_blob_hash,
			Err(_) => return Ok(bad_request()),
		};

		// Get the blob.
		let handle = match self.get_blob(blob_hash).await? {
			Some(handle) => handle,
			None => return Ok(not_found()),
		};

		// Create the stream for the file.
		let file = tokio::fs::File::open(&handle.path).await.with_context(|| {
			format!(
				r#"Failed to open file at path "{}"."#,
				&handle.path.display()
			)
		})?;
		let stream = tokio_util::io::ReaderStream::new(file);
		let response = hyper::Body::wrap_stream(stream);

		// Create the response.
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(response)
			.unwrap();

		Ok(response)
	}
}
