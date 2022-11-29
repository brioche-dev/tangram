use crate::{builder::State, checksum::Checksummer, expression::Download, hash::Hash};
use anyhow::{anyhow, Result};
use futures::{Stream, StreamExt, TryStreamExt};
use std::{
	path::Path,
	sync::{Arc, Mutex},
};
use tokio_util::io::{StreamReader, SyncIoBridge};

impl State {
	pub(super) async fn evaluate_download(&self, _hash: Hash, download: &Download) -> Result<Hash> {
		// Acquire a file system permit.
		let _permit = self.file_system_semaphore.acquire().await?;

		// Get the archive format.
		let archive_format = ArchiveFormat::for_path(Path::new(download.url.path()));

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

		// Create the checksummer.
		let checker = Checksummer::new(download.checksum.clone());
		let checker = Arc::new(Mutex::new(checker));

		// Compute the checksum while streaming.
		let stream = {
			let checker = checker.clone();
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
		let checker = Arc::try_unwrap(checker).unwrap().into_inner()?;
		checker.finalize_and_validate().map_err(|error| {
			anyhow!(r#"Error downloading from URL "{}".\n{error}"#, download.url)
		})?;

		Ok(artifact)
	}
}

impl State {
	async fn download_simple<S>(&self, stream: S) -> Result<Hash>
	where
		S: Stream<Item = std::io::Result<hyper::body::Bytes>> + Send + Unpin + 'static,
	{
		// Create a temp path.
		let temp_path = self.create_temp_path();

		// Read the stream to the temp path.
		{
			let mut reader = StreamReader::new(stream);
			let mut file = tokio::fs::File::create(&temp_path).await?;
			let mut file_writer = tokio::io::BufWriter::new(&mut file);
			tokio::io::copy(&mut reader, &mut file_writer).await?;
		}

		// Checkin the temp path.
		let artifact = self.checkin(&temp_path).await?;

		Ok(artifact)
	}

	async fn download_tar_unpack<S>(
		&self,
		stream: S,
		compression_format: Option<CompressionFormat>,
	) -> Result<Hash>
	where
		S: Stream<Item = std::io::Result<hyper::body::Bytes>> + Send + Unpin + 'static,
	{
		// Create a temp path to unpack to.
		let unpack_temp_path = self.create_temp_path();

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

		Ok(artifact)
	}

	async fn download_zip_unpack<S>(&self, stream: S) -> Result<Hash>
	where
		S: Stream<Item = std::io::Result<hyper::body::Bytes>> + Send + Unpin + 'static,
	{
		// Create a temp path.
		let temp_path = self.create_temp_path();

		// Read the stream to the temp path.
		{
			let mut reader = StreamReader::new(stream);
			let mut file = tokio::fs::File::create(&temp_path).await?;
			let mut file_writer = tokio::io::BufWriter::new(&mut file);
			tokio::io::copy(&mut reader, &mut file_writer).await?;
		}

		// Create a temp path to unpack to.
		let unpack_temp_path = self.create_temp_path();

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

		Ok(artifact)
	}
}

enum ArchiveFormat {
	Tar(Option<CompressionFormat>),
	Zip,
}

impl ArchiveFormat {
	#[allow(clippy::case_sensitive_file_extension_comparisons)]
	pub fn for_path(path: &Path) -> Option<ArchiveFormat> {
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

enum CompressionFormat {
	Bz2,
	Gz,
	Lz,
	Xz,
	Zstd,
}
