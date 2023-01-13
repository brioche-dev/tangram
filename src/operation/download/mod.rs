use self::archive_format::{ArchiveFormat, CompressionFormat};
use crate::{
	checksum::{Algorithm, Checksum, Checksummer},
	value::Value,
	Cli,
};
use anyhow::{bail, Result};
use futures::{Stream, StreamExt, TryStreamExt};
use std::{
	path::Path,
	sync::{Arc, Mutex},
};
use tokio_util::io::{StreamReader, SyncIoBridge};
use url::Url;

mod archive_format;

#[derive(
	Clone, Debug, buffalo::Deserialize, buffalo::Serialize, serde::Deserialize, serde::Serialize,
)]
pub struct Download {
	#[buffalo(id = 0)]
	pub url: Url,

	#[buffalo(id = 1)]
	pub unpack: bool,

	#[buffalo(id = 2)]
	pub checksum: Option<Checksum>,

	#[buffalo(id = 3)]
	#[serde(default, rename = "unsafe")]
	pub is_unsafe: bool,
}

impl Cli {
	pub(super) async fn run_download(&self, download: &Download) -> Result<Value> {
		// Acquire a file system permit.
		let _permit = self.state.file_system_semaphore.acquire().await?;

		// Get the archive format.
		let archive_format = ArchiveFormat::for_path(Path::new(download.url.path()));

		// Send the request.
		let response = self
			.state
			.http_client
			.get(download.url.clone())
			.send()
			.await?
			.error_for_status()?;

		// Get a stream for the response body.
		let stream = response
			.bytes_stream()
			.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error));

		// Create the checksummer.
		let algorithm = download
			.checksum
			.as_ref()
			.map_or(Algorithm::Sha256, Checksum::algorithm);
		let checksummer = Checksummer::new(algorithm);
		let checksummer = Arc::new(Mutex::new(checksummer));

		// Compute the checksum while streaming.
		let stream = {
			let checker = checksummer.clone();
			stream.map(move |value| {
				let value = value?;
				checker.lock().unwrap().update(&value);
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
			let checksummer = Arc::try_unwrap(checksummer).unwrap().into_inner().unwrap();
			let checksum = checksummer.finalize();

			// Ensure a checksum was provided.
			let Some(expected) = download.checksum.as_ref() else {
				bail!(r#"No checksum was provided. The checksum was "{checksum:?}"."#);
			};

			// Verify the checksum.
			if &checksum != expected {
				bail!(
					r#"The checksum did not match. Expected "{expected:?}" but got "{checksum:?}"."#
				);
			}
		}

		Ok(artifact)
	}
}

impl Cli {
	async fn download_simple<S>(&self, stream: S) -> Result<Value>
	where
		S: Stream<Item = std::io::Result<hyper::body::Bytes>> + Send + Unpin + 'static,
	{
		// Create a temp path.
		let temp_path = self.temp_path();

		// Read the stream to the temp path.
		{
			let mut reader = StreamReader::new(stream);
			let mut file = tokio::fs::File::create(&temp_path).await?;
			let mut file_writer = tokio::io::BufWriter::new(&mut file);
			tokio::io::copy(&mut reader, &mut file_writer).await?;
		}

		// Checkin the temp path.
		let artifact = self.checkin(&temp_path).await?;

		// Create the artifact value.
		let artifact = Value::Artifact(artifact);

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
		// Create a temp path to unpack to.
		let unpack_temp_path = self.temp_path();

		// Stream and unpack simultaneously in a blocking task.
		tokio::task::spawn_blocking({
			let reader = StreamReader::new(stream);
			let reader = SyncIoBridge::new(reader);
			let unpack_temp_path = unpack_temp_path.clone();
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
				archive.unpack(&unpack_temp_path)?;
				Ok(())
			}
		})
		.await
		.unwrap()?;

		// Check in the unpack temp path.
		let artifact = self.checkin(&unpack_temp_path).await?;

		// Create the artifact value.
		let artifact = Value::Artifact(artifact);

		Ok(artifact)
	}

	async fn download_zip_unpack<S>(&self, stream: S) -> Result<Value>
	where
		S: Stream<Item = std::io::Result<hyper::body::Bytes>> + Send + Unpin + 'static,
	{
		// Create a temp path.
		let temp_path = self.temp_path();

		// Read the stream to the temp path.
		{
			let mut reader = StreamReader::new(stream);
			let mut file = tokio::fs::File::create(&temp_path).await?;
			let mut file_writer = tokio::io::BufWriter::new(&mut file);
			tokio::io::copy(&mut reader, &mut file_writer).await?;
		}

		// Create a temp path to unpack to.
		let unpack_temp_path = self.temp_path();

		// Unpack in a blocking task.
		tokio::task::spawn_blocking({
			let unpack_temp_path = unpack_temp_path.clone();
			move || -> Result<_> {
				let archive_file = std::fs::File::open(temp_path)?;
				let archive_reader = std::io::BufReader::new(archive_file);
				let mut zip = zip::ZipArchive::new(archive_reader)?;
				zip.extract(&unpack_temp_path)?;
				Ok(())
			}
		})
		.await
		.unwrap()?;

		// Check in the unpack temp path.
		let artifact = self.checkin(&unpack_temp_path).await?;

		// Create the artifact value.
		let artifact = Value::Artifact(artifact);

		Ok(artifact)
	}
}
