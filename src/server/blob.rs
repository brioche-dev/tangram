use super::{error::bad_request, Server};
use crate::{hash::Hasher, object::BlobHash, util::path_exists};
use anyhow::{bail, Context, Result};
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

impl Server {
	pub(super) async fn handle_create_blob_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let client_blob_hash = if let ["blobs", blob_hash] = path_components.as_slice() {
			blob_hash
		} else {
			bail!("Unexpected path.")
		};
		let client_blob_hash = match client_blob_hash.parse() {
			Ok(client_blob_hash) => client_blob_hash,
			Err(_) => return Ok(bad_request()),
		};

		// Read and deserialize the request body.
		let body = hyper::body::to_bytes(request.into_body())
			.await
			.context("Failed to read the request body.")?;

		let bytes: &[u8] = &body;
		let server_blob_hash = self.add_blob_from_reader(bytes).await?;
		if server_blob_hash != client_blob_hash {
			bail!("The blob hash is not correct for the given bytes.");
		}

		// Create the response.
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::empty())
			.unwrap();

		Ok(response)
	}
}
