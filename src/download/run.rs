use super::{unpack, Download};
use crate::{
	checksum::{self, Checksum},
	error::{return_error, Error, Result, WrapErr},
	temp::Temp,
	util::fs,
	value::Value,
	Instance,
};
use futures::{Stream, StreamExt, TryStreamExt};
use std::sync::{Arc, Mutex};
use tokio_util::io::{StreamReader, SyncIoBridge};

impl Instance {
	#[tracing::instrument(skip_all, fields(url = %download.url, unpack = download.unpack, is_unsafe = download.is_unsafe))]
	pub async fn run_download(&self, download: &Download) -> Result<Value> {
<<<<<<< HEAD
		// Acquire a file permit.
		let _file_permit = self.file_semaphore.acquire().await.map_err(Error::other)?;

		tracing::debug!("Acquired file permit.");

=======
>>>>>>> 5ebaaff (Implement hostPaths for processes and move out the LSP and server to its own struct.)
		// Acquire a socket permit.
		let _socket_permit = self
			.socket_semaphore
			.acquire()
			.await
			.map_err(Error::other)?;

		tracing::debug!("Acquired socket permit.");

		// Get the unpack format.
		let unpack_format = if download.unpack {
			Some(
				unpack::Format::for_path(fs::Path::new(download.url.path()))
					.wrap_err("Failed to determine the unpack format.")?,
			)
		} else {
			None
		};

		tracing::info!(?download.url, ?unpack_format, "Downloading artifact.");

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
			stream.map(move |value| -> std::io::Result<_> {
				let value = value?;
				checksum_writer.lock().unwrap().update(&value);
				Ok(value)
			})
		};

		let artifact = match unpack_format {
			None => self.download_simple(stream).await?,

			Some(unpack::Format::Tar(compression)) => {
				self.download_tar(stream, compression).await?
			},

			Some(unpack::Format::Zip) => self.download_zip(stream).await?,
		};

		tracing::info!(?download.url, "Downloaded artifact.");

		// If the download is not unsafe, then verify the checksum.
		if !download.is_unsafe {
			// Finalize the checksum.
			let checksum_writer = Arc::try_unwrap(checksum_writer)
				.unwrap()
				.into_inner()
				.unwrap();
			let actual = checksum_writer.finalize();

			// Ensure a checksum was provided.
			let Some(expected) = download.checksum.clone() else {
				return_error!(r#"No checksum was provided. The checksum was "{actual}"."#);
			};

			// Verify the checksum.
			if expected != actual {
				return_error!(
					r#"The checksum did not match. Expected "{expected}" but got "{actual}"."#
				);
			}

			tracing::debug!("Validated checksums.");
		}

		Ok(artifact)
	}
}

impl Instance {
	#[tracing::instrument(skip_all)]
	async fn download_simple<S>(&self, stream: S) -> Result<Value>
	where
		S: Stream<Item = std::io::Result<hyper::body::Bytes>> + Send + Unpin + 'static,
	{
		// Create a temp.
		let temp = Temp::new(self);

		tracing::debug!(temp_path = ?temp.path(), "Performing simple download.");

		// Read the stream to the temp.
		let mut reader = StreamReader::new(stream);
		let mut file = tokio::fs::File::create(temp.path()).await?;
		tokio::io::copy(&mut reader, &mut file).await?;
		drop(file);

		// Check in the temp.
		let artifact_hash = self
			.check_in(temp.path())
			.await
			.wrap_err("Failed to check in the temp path.")?;

		tracing::debug!(?artifact_hash, "Checked in simple download.");

		// Create the artifact value.
		let artifact = Value::Artifact(artifact_hash);

		Ok(artifact)
	}

	#[tracing::instrument(skip_all)]
	async fn download_tar<S>(
		&self,
		stream: S,
		compression: Option<unpack::Compression>,
	) -> Result<Value>
	where
		S: Stream<Item = std::io::Result<hyper::body::Bytes>> + Send + Unpin + 'static,
	{
		// Create a temp.
		let temp = Temp::new(self);

		tracing::debug!(temp_path = ?temp.path(), ?compression, "Performing tar download.");

		// Stream and unpack simultaneously in a blocking task.
		tokio::task::spawn_blocking({
			let reader = StreamReader::new(stream);
			let reader = SyncIoBridge::new(reader);
			let temp_path = temp.path().to_owned();
			let span = tracing::info_span!("download_tar_spawn_blocking");
			move || -> Result<_> {
				let _enter = span.enter();
				tracing::debug!("Started tar task.");
				let archive_reader = std::io::BufReader::new(reader);
				let archive_reader: Box<dyn std::io::Read + Send> = match compression {
					None => Box::new(archive_reader),
					Some(unpack::Compression::Bz2) => {
						Box::new(bzip2::read::BzDecoder::new(archive_reader))
					},
					Some(unpack::Compression::Gz) => {
						Box::new(flate2::read::GzDecoder::new(archive_reader))
					},
					Some(unpack::Compression::Lz) => {
						Box::new(lz4_flex::frame::FrameDecoder::new(archive_reader))
					},
					Some(unpack::Compression::Xz) => {
						Box::new(xz::read::XzDecoder::new(archive_reader))
					},
					Some(unpack::Compression::Zstd) => {
						Box::new(zstd::Decoder::new(archive_reader)?)
					},
				};
				let mut archive = tar::Archive::new(archive_reader);
				archive.set_preserve_permissions(false);
				archive.set_unpack_xattrs(false);
				archive.unpack(temp_path)?;
				tracing::debug!("Finished tar task.");
				Ok(())
			}
		})
		.await
		.unwrap()?;

		// Check in the temp.
		let artifact_hash = self
			.check_in(temp.path())
			.await
			.wrap_err("Failed to check in the temp path.")?;

		tracing::debug!(?artifact_hash, "Checked in tar download.");

		// Create the artifact value.
		let artifact = Value::Artifact(artifact_hash);

		Ok(artifact)
	}

	async fn download_zip<S>(&self, stream: S) -> Result<Value>
	where
		S: Stream<Item = std::io::Result<hyper::body::Bytes>> + Send + Unpin + 'static,
	{
		// Create a temp.
		let temp = Temp::new(self);

		tracing::debug!(temp_path = ?temp.path(), "Performing zip download.");

		// Read the stream to the temp path.
		let mut reader = StreamReader::new(stream);
		let mut file = tokio::fs::File::create(temp.path()).await?;
		tokio::io::copy(&mut reader, &mut file).await?;
		drop(file);

		// Create a temp to unpack to.
		let unpack_temp = Temp::new(self);

		// Unpack in a blocking task.
		tokio::task::spawn_blocking({
			let temp_path = temp.path().to_owned();
			let unpack_temp_path = unpack_temp.path().to_owned();
			move || -> Result<_> {
				let archive_file =
					std::fs::File::open(&temp_path).wrap_err("Failed to open the zip archive.")?;
				let archive_reader = std::io::BufReader::new(archive_file);
				let mut zip = zip::ZipArchive::new(archive_reader)
					.map_err(Error::other)
					.wrap_err("Failed to read the zip archive.")?;
				zip.extract(&unpack_temp_path)
					.map_err(Error::other)
					.wrap_err("Failed to extract the zip archive.")?;
				Ok(())
			}
		})
		.await
		.unwrap()?;

		// Check in the unpack temp path.
		let artifact_hash = self
			.check_in(unpack_temp.path())
			.await
			.wrap_err("Failed to check in the temp path.")?;

		tracing::debug!(?artifact_hash, "Checked in zip download.");

		// Create the artifact value.
		let artifact = Value::Artifact(artifact_hash);

		Ok(artifact)
	}
}
