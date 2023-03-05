use super::{ArchiveFormat, CompressionFormat, Download};
use crate::{
	checksum::{self, Checksum},
	os,
	value::Value,
	Instance,
};
use anyhow::{bail, Context, Result};
use futures::{Stream, StreamExt, TryStreamExt};
use std::sync::{Arc, Mutex};
use tokio_util::io::{StreamReader, SyncIoBridge};

impl Instance {
	pub async fn run_download(&self, download: &Download) -> Result<Value> {
		// Acquire a file permit.
		let _file_permit = self.file_semaphore.acquire().await?;

		// Acquire a socket permit.
		let _socket_permit = self.socket_semaphore.acquire().await?;

		// Get the archive format.
		let archive_format = ArchiveFormat::for_path(os::Path::new(download.url.path()));

		// Send the request.
		let response = self
			.http_client
			.get(download.url.clone())
			.send()
			.await?
			.error_for_status()?;

		// Get a stream for the response body.
		let stream = response
			.bytes_stream()
			.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error));

		// Create the checksum writer.
		let algorithm = download
			.checksum
			.as_ref()
			.map_or(checksum::Algorithm::Sha256, Checksum::algorithm);
		let checksum_writer = checksum::Writer::new(algorithm);
		let checksum_writer = Arc::new(Mutex::new(checksum_writer));

		// Compute the checksum while streaming.
		let stream = {
			let checksum_writer = checksum_writer.clone();
			stream.map(move |value| {
				let value = value?;
				checksum_writer.lock().unwrap().update(&value);
				Ok::<_, std::io::Error>(value)
			})
		};

		let artifact = match (download.unpack, archive_format) {
			(_, None) | (false, _) => self.download_simple(stream).await?,

			(true, Some(ArchiveFormat::Tar(compression_format))) => {
				self.download_tar_unpack(stream, compression_format).await?
			},

			(true, Some(ArchiveFormat::Zip)) => self.download_zip_unpack(stream).await?,
		};

		// Verify the checksum.
		if !download.is_unsafe {
			// Finalize the checksum.
			let checksum_writer = Arc::try_unwrap(checksum_writer)
				.unwrap()
				.into_inner()
				.unwrap();
			let actual = checksum_writer.finalize();

			// Ensure a checksum was provided.
			let Some(expected) = download.checksum.clone() else {
				bail!(r#"No checksum was provided. The checksum was "{actual:?}"."#);
			};

			// Verify the checksum.
			if expected != actual {
				bail!(
					r#"The checksum did not match. Expected "{expected:?}" but got "{actual:?}"."#
				);
			}
		}

		Ok(artifact)
	}
}

impl Instance {
	async fn download_simple<S>(&self, stream: S) -> Result<Value>
	where
		S: Stream<Item = std::io::Result<hyper::body::Bytes>> + Send + Unpin + 'static,
	{
		// Create a temp path.
		let temp_path = self.temp_path();

		// Read the stream to the temp path.
		let mut reader = StreamReader::new(stream);
		let mut file = tokio::fs::File::create(&temp_path).await?;
		tokio::io::copy(&mut reader, &mut file).await?;
		drop(file);

		// Check in the temp path.
		let artifact_hash = self
			.check_in(&temp_path)
			.await
			.context("Failed to check in the temp path.")?;

		// Remove the temp path.
		tokio::fs::remove_file(&temp_path)
			.await
			.context("Failed to remove the temp path.")?;

		// Create the artifact value.
		let artifact = Value::Artifact(artifact_hash);

		Ok(artifact)
	}

	async fn download_tar_unpack<S>(
		&self,
		stream: S,
		compression_format: Option<CompressionFormat>,
	) -> Result<Value>
	where
		S: Stream<Item = std::io::Result<hyper::body::Bytes>> + Send + Unpin + 'static,
	{
		// Create a temp path.
		let temp_path = self.temp_path();

		// Stream and unpack simultaneously in a blocking task.
		tokio::task::spawn_blocking({
			let reader = StreamReader::new(stream);
			let reader = SyncIoBridge::new(reader);
			let temp_path = temp_path.clone();
			move || -> Result<_> {
				let archive_reader = std::io::BufReader::new(reader);
				let archive_reader: Box<dyn std::io::Read + Send> = match compression_format {
					None => Box::new(archive_reader),
					Some(CompressionFormat::Bz2) => {
						Box::new(bzip2::read::BzDecoder::new(archive_reader))
					},
					Some(CompressionFormat::Gz) => {
						Box::new(flate2::read::GzDecoder::new(archive_reader))
					},
					Some(CompressionFormat::Lz) => {
						Box::new(lz4_flex::frame::FrameDecoder::new(archive_reader))
					},
					Some(CompressionFormat::Xz) => {
						Box::new(xz::read::XzDecoder::new(archive_reader))
					},
					Some(CompressionFormat::Zstd) => Box::new(zstd::Decoder::new(archive_reader)?),
				};
				let mut archive = tar::Archive::new(archive_reader);
				archive.set_preserve_permissions(false);
				archive.set_unpack_xattrs(false);
				archive.unpack(&temp_path)?;
				Ok(())
			}
		})
		.await
		.unwrap()?;

		// Check in the temp path.
		let artifact_hash = self
			.check_in(&temp_path)
			.await
			.context("Failed to check in the temp path.")?;

		// Remove the temp path.
		os::fs::rmrf(&temp_path, None)
			.await
			.context("Failed to remove the temp path.")?;

		// Create the artifact value.
		let artifact = Value::Artifact(artifact_hash);

		Ok(artifact)
	}

	async fn download_zip_unpack<S>(&self, stream: S) -> Result<Value>
	where
		S: Stream<Item = std::io::Result<hyper::body::Bytes>> + Send + Unpin + 'static,
	{
		// Create a temp path.
		let temp_path = self.temp_path();

		// Read the stream to the temp path.
		let mut reader = StreamReader::new(stream);
		let mut file = tokio::fs::File::create(&temp_path).await?;
		tokio::io::copy(&mut reader, &mut file).await?;
		drop(file);

		// Create a temp path to unpack to.
		let unpack_temp_path = self.temp_path();

		// Unpack in a blocking task.
		tokio::task::spawn_blocking({
			let temp_path = temp_path.clone();
			let unpack_temp_path = unpack_temp_path.clone();
			move || -> Result<_> {
				let archive_file =
					std::fs::File::open(&temp_path).context("Failed to open the zip archive.")?;
				let archive_reader = std::io::BufReader::new(archive_file);
				let mut zip = zip::ZipArchive::new(archive_reader)
					.context("Failed to read the zip archive.")?;
				zip.extract(&unpack_temp_path)
					.context("Failed to extract the zip archive.")?;
				Ok(())
			}
		})
		.await
		.unwrap()?;

		// Remove the temp path.
		tokio::fs::remove_file(&temp_path)
			.await
			.context("Failed to remove the temp path.")?;

		// Check in the unpack temp path.
		let artifact_hash = self
			.check_in(&unpack_temp_path)
			.await
			.context("Failed to check in the .")?;

		// Remove the unpack temp path.
		os::fs::rmrf(&unpack_temp_path, None)
			.await
			.context("Failed to remove the unpack temp path.")?;

		// Create the artifact value.
		let artifact = Value::Artifact(artifact_hash);

		Ok(artifact)
	}
}

impl ArchiveFormat {
	#[allow(clippy::case_sensitive_file_extension_comparisons)]
	#[must_use]
	pub fn for_path(path: &os::Path) -> Option<ArchiveFormat> {
		let path = path.to_str().unwrap();
		if path.ends_with(".tar.bz2") || path.ends_with(".tbz2") {
			Some(ArchiveFormat::Tar(Some(CompressionFormat::Bz2)))
		} else if path.ends_with(".tar.gz") || path.ends_with(".tgz") {
			Some(ArchiveFormat::Tar(Some(CompressionFormat::Gz)))
		} else if path.ends_with(".tar.lz") || path.ends_with(".tlz") {
			Some(ArchiveFormat::Tar(Some(CompressionFormat::Lz)))
		} else if path.ends_with(".tar.xz") || path.ends_with(".txz") {
			Some(ArchiveFormat::Tar(Some(CompressionFormat::Xz)))
		} else if path.ends_with(".tar.zstd")
			|| path.ends_with(".tzstd")
			|| path.ends_with(".tar.zst")
			|| path.ends_with(".tzst")
		{
			Some(ArchiveFormat::Tar(Some(CompressionFormat::Zstd)))
		} else if path.ends_with(".tar") {
			Some(ArchiveFormat::Tar(None))
		} else if path.ends_with(".zip") {
			Some(ArchiveFormat::Zip)
		} else {
			None
		}
	}
}
