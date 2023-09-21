use crate::{return_error, Checksum, Error, Result, Server, Temp, Value, WrapErr};
use bytes::Bytes;
use futures::{Stream, StreamExt, TryStreamExt};
use std::sync::{Arc, Mutex};
use tokio_util::io::{StreamReader, SyncIoBridge};

impl Resource {
	#[tracing::instrument(skip(tg))]
	pub async fn download(&self, server: &Server) -> Result<Value> {
		let operation = Build::Resource(self.clone());
		operation.output(server, None).await
	}

	pub(crate) async fn download_inner(&self, server: &Server) -> Result<Value> {
		tracing::info!(?self.url, "Downloading.");

		// Send the request.
		let response = server
			.http_client
			.get(self.url.clone())
			.send()
			.await?
			.error_for_status()?;

		// Get a stream for the response body.
		let stream = response
			.bytes_stream()
			.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error));

		// Create the checksum writer.
		let algorithm = self
			.checksum
			.as_ref()
			.map_or(checksum::Algorithm::Sha256, Checksum::algorithm);
		let checksum_writer = checksum::Writer::new(algorithm);
		let checksum_writer = Arc::new(Mutex::new(checksum_writer));

		// Compute the checksum while streaming.
		let stream = {
			let checksum_writer = checksum_writer.clone();
			stream.map(move |bytes| -> std::io::Result<_> {
				let bytes = bytes?;
				checksum_writer.lock().unwrap().update(&bytes);
				Ok(bytes)
			})
		};

		let artifact = match self.unpack {
			None => Self::download_simple(tg, stream).await?,
			Some(unpack::Format::Tar) => Self::download_tar(tg, stream, None).await?,
			Some(unpack::Format::TarBz2) => {
				Self::download_tar(tg, stream, Some(Compression::Bz2)).await?
			},
			Some(unpack::Format::TarGz) => {
				Self::download_tar(tg, stream, Some(Compression::Gz)).await?
			},
			Some(unpack::Format::TarXz) => {
				Self::download_tar(tg, stream, Some(Compression::Xz)).await?
			},
			Some(unpack::Format::TarZstd) => {
				Self::download_tar(tg, stream, Some(Compression::Zstd)).await?
			},
			Some(unpack::Format::Zip) => Self::download_zip(tg, stream).await?,
		};

		tracing::info!(?self.url, "Downloaded.");

		// If the download is not unsafe, then verify the checksum.
		if !self.unsafe_ {
			// Finalize the checksum.
			let checksum_writer = Arc::try_unwrap(checksum_writer)
				.unwrap()
				.into_inner()
				.unwrap();
			let actual = checksum_writer.finalize();

			// Ensure a checksum was provided.
			let Some(expected) = self.checksum.clone() else {
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

	#[tracing::instrument(skip_all)]
	async fn download_simple<S>(server: &Server, stream: S) -> Result<Value>
	where
		S: Stream<Item = std::io::Result<Bytes>> + Send + Unpin + 'static,
	{
		// Create a temp.
		let temp = Temp::new(tg);

		tracing::debug!(temp_path = ?temp.path(), "Performing simple download.");

		// Read the stream to the temp.
		let mut reader = StreamReader::new(stream);
		let mut file = tokio::fs::File::create(temp.path()).await?;
		tokio::io::copy(&mut reader, &mut file).await?;
		drop(file);

		// Check in the temp.
		let artifact = Artifact::check_in(tg, temp.path())
			.await
			.wrap_err("Failed to check in the temp path.")?;

		tracing::debug!(?artifact, "Checked in simple download.");

		// Create the value.
		let value = Value::Artifact(artifact);

		Ok(value)
	}

	async fn download_tar<S>(
		server: &Server,
		stream: S,
		compression: Option<unpack::Compression>,
	) -> Result<Value>
	where
		S: Stream<Item = std::io::Result<Bytes>> + Send + Unpin + 'static,
	{
		// Create a temp.
		let temp = Temp::new(tg);

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
					Some(unpack::Compression::Xz) => {
						Box::new(xz2::read::XzDecoder::new(archive_reader))
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
		let artifact = Artifact::check_in(tg, temp.path())
			.await
			.wrap_err("Failed to check in the temp path.")?;

		tracing::debug!(?artifact, "Checked in tar download.");

		// Create the value.
		let value = Value::Artifact(artifact);

		Ok(value)
	}

	async fn download_zip<S>(server: &Server, stream: S) -> Result<Value>
	where
		S: Stream<Item = std::io::Result<Bytes>> + Send + Unpin + 'static,
	{
		// Create a temp.
		let temp = Temp::new(tg);

		tracing::debug!(temp_path = ?temp.path(), "Performing zip download.");

		// Read the stream to the temp path.
		let mut reader = StreamReader::new(stream);
		let mut file = tokio::fs::File::create(temp.path()).await?;
		tokio::io::copy(&mut reader, &mut file).await?;
		drop(file);

		// Create a temp to unpack to.
		let unpack_temp = Temp::new(tg);

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
		let artifact = Artifact::check_in(tg, unpack_temp.path())
			.await
			.wrap_err("Failed to check in the temp path.")?;

		tracing::debug!(?artifact, "Checked in zip download.");

		// Create the value.
		let value = Value::Artifact(artifact);

		Ok(value)
	}
}
