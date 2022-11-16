use crate::{builder::State, digest::Hasher, expression::Fetch, hash::Hash};
use anyhow::{anyhow, Result};
use futures::{StreamExt, TryStreamExt};
use std::{
	path::Path,
	sync::{Arc, Mutex},
};
use tokio_util::io::{StreamReader, SyncIoBridge};

impl State {
	pub(super) async fn evaluate_fetch(&self, _hash: Hash, fetch: &Fetch) -> Result<Hash> {
		tracing::trace!(r#"Fetching "{}"."#, fetch.url);

		// Get the archive format.
		let archive_format = ArchiveFormat::for_path(Path::new(fetch.url.path()));

		// Create a temp path.
		let fetch_temp_path = self.create_temp_path();

		// Send the request.
		let response = self
			.http_client
			.get(fetch.url.clone())
			.send()
			.await?
			.error_for_status()?;

		tracing::trace!(r#"Fetched "{}"."#, fetch.url);

		// Get a reader for the response body.
		let stream = response
			.bytes_stream()
			.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error));

		let artifact = match (fetch.unpack, archive_format) {
			(_, None) | (false, _) => self.fetch_simple(fetch, &fetch_temp_path, stream).await?,

			(true, Some(ArchiveFormat::Tar(compression_format))) => {
				self.fetch_tar_unpack(fetch, compression_format, stream)
					.await?
			},

			(true, Some(ArchiveFormat::Zip)) => {
				self.fetch_zip_unpack(fetch, &fetch_temp_path, stream)
					.await?
			},
		};

		Ok(artifact)
	}
}

impl State {
	async fn fetch_zip_unpack<
		S: futures::Stream<Item = futures::io::Result<hyper::body::Bytes>> + Unpin + 'static,
	>(
		&self,
		fetch: &Fetch,
		fetch_temp_path: &Path,
		stream: S,
	) -> Result<Hash> {
		// Create the hasher.
		let hasher = Hasher::new(fetch.digest.clone());
		let hasher = Arc::new(Mutex::new(hasher));

		// Map the stream to support hash and copy in one pass.
		let stream = {
			let hasher = hasher.clone();
			stream.map(move |value| {
				let value = value?;
				hasher.lock().unwrap().update(&value);
				Ok::<_, std::io::Error>(value)
			})
		};

		// Create a reader and copy the bytes from the stream.
		let mut reader = StreamReader::new(stream);
		let mut file = tokio::fs::File::create(&fetch_temp_path).await?;
		let mut file_writer = tokio::io::BufWriter::new(&mut file);
		tokio::io::copy(&mut reader, &mut file_writer).await?;

		// Verify the hash.
		let hasher = Arc::try_unwrap(hasher).unwrap().into_inner()?;
		hasher
			.finalize_and_validate()
			.map_err(|error| anyhow!("Error fetching URL {}: {error}.", fetch.url.clone()))?;

		tracing::trace!(r#"Unpacking the contents of URL "{}"."#, fetch.url);

		// Create a temp to unpack to.
		let unpack_temp_path = self.create_temp_path();

		// Unpack in a blocking task.
		tokio::task::spawn_blocking({
			let unpack_temp_path = unpack_temp_path.clone();
			let fetch_temp_path = fetch_temp_path.to_owned();
			move || -> Result<_> {
				let archive_file = std::fs::File::open(fetch_temp_path)?;
				let archive_reader = std::io::BufReader::new(archive_file);
				let mut zip = zip::ZipArchive::new(archive_reader)?;
				zip.extract(&unpack_temp_path)?;
				Ok(())
			}
		})
		.await
		.unwrap()?;

		// Check in the temp.
		let artifact = self.checkin(&unpack_temp_path).await?;

		tracing::trace!(
			r#"Unpacked the contents of URL "{}" to artifact "{}"."#,
			fetch.url,
			artifact,
		);

		Ok(artifact)
	}

	async fn fetch_tar_unpack<
		S: futures::Stream<Item = futures::io::Result<hyper::body::Bytes>> + Send + Unpin + 'static,
	>(
		&self,
		fetch: &Fetch,
		compression_format: Option<CompressionFormat>,
		stream: S,
	) -> Result<Hash> {
		// Create the hasher.
		let hasher = Hasher::new(fetch.digest.clone());
		let hasher = Arc::new(Mutex::new(hasher));

		// Map the stream to support hash and copy in one pass.
		let stream = {
			let hasher = hasher.clone();
			stream.map(move |value| {
				let value = value?;
				hasher.lock().unwrap().update(&value);
				Ok::<_, std::io::Error>(value)
			})
		};
		let reader = StreamReader::new(stream);
		let reader = SyncIoBridge::new(reader);

		tracing::trace!(r#"Unpacking the contents of URL "{}"."#, fetch.url);

		// Create a temp to unpack to.
		let unpack_temp_path = self.create_temp_path();

		// Unpack in a blocking task.
		tokio::task::spawn_blocking({
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

		// Verify the hash.
		let hasher = Arc::try_unwrap(hasher).unwrap().into_inner()?;
		hasher
			.finalize_and_validate()
			.map_err(|error| anyhow!("Error fetching URL {}: {error}.", fetch.url.clone()))?;

		// Check in the temp.
		let artifact = self.checkin(&unpack_temp_path).await?;

		tracing::trace!(
			r#"Unpacked the contents of URL "{}" to artifact "{}"."#,
			fetch.url,
			artifact,
		);

		Ok(artifact)
	}

	async fn fetch_simple<
		S: futures::Stream<Item = futures::io::Result<hyper::body::Bytes>> + Unpin,
	>(
		&self,
		fetch: &Fetch,
		fetch_temp_path: &Path,
		stream: S,
	) -> Result<Hash> {
		// Create the hasher.
		let hasher = Hasher::new(fetch.digest.clone());
		let hasher = Arc::new(Mutex::new(hasher));

		// Map the stream to support hash and copy in one pass.
		let stream = {
			let hasher = Arc::clone(&hasher);
			stream.map(move |value| {
				let value = value?;
				hasher.lock().unwrap().update(&value);
				Ok::<_, std::io::Error>(value)
			})
		};

		// Create a reader and copy the bytes to the temp.
		{
			let mut reader = StreamReader::new(stream);
			let mut file = tokio::fs::File::create(fetch_temp_path).await?;
			let mut file_writer = tokio::io::BufWriter::new(&mut file);
			tokio::io::copy(&mut reader, &mut file_writer).await?;
		}

		// Verify the hash.
		let hasher = Arc::try_unwrap(hasher).unwrap().into_inner()?;
		hasher
			.finalize_and_validate()
			.map_err(|error| anyhow!("Error fetching URL {}: {error}.", fetch.url.clone()))?;

		// Checkin the temp.
		let artifact = self.checkin(fetch_temp_path).await?;

		Ok(artifact)
	}
}

enum ArchiveFormat {
	Tar(Option<CompressionFormat>),
	Zip,
}

enum CompressionFormat {
	Bz2,
	Gz,
	Lz,
	Xz,
	Zstd,
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
