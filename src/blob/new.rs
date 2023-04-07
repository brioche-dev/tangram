use super::{Blob, Hash};
use crate::{error::Result, hash::Writer, instance::Instance, temp::Temp};
use tokio::io::{AsyncRead, AsyncWriteExt};
use tokio_stream::StreamExt;

impl Blob {
	pub async fn new(tg: &Instance, reader: impl AsyncRead + Unpin) -> Result<Blob> {
		// Create a temp file to read the blob into.
		let temp = Temp::new(tg);
		let mut temp_file = tokio::fs::File::create(temp.path()).await?;

		// Compute the hash of the bytes in the reader and write the bytes to the temp file.
		let mut stream = tokio_util::io::ReaderStream::new(reader);
		let mut hash_writer = Writer::new();
		while let Some(chunk) = stream.next().await {
			let chunk = chunk?;
			hash_writer.update(&chunk);
			temp_file.write_all(&chunk).await?;
		}
		let hash = Hash(hash_writer.finalize());

		// Close the temp file.
		temp_file.sync_all().await?;
		drop(temp_file);

		// Move the temp file to the blobs path.
		let blob_path = tg.blob_path(hash);
		tokio::fs::rename(temp.path(), &blob_path).await?;

		// Create the blob.
		let blob = Blob { hash };

		Ok(blob)
	}
}
