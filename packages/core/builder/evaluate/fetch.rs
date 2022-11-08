use crate::{builder::State, digest::Hasher, expression::Fetch, hash::Hash};
use anyhow::{anyhow, Context, Result};
use futures::{StreamExt, TryStreamExt};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

impl State {
	pub(super) async fn evaluate_fetch(&self, _hash: Hash, fetch: &Fetch) -> Result<Hash> {
		tracing::trace!(r#"Fetching "{}"."#, fetch.url);

		// Create a temp path.
		let temp_path = self.create_temp_path();

		// Send the request.
		let response = self
			.http_client
			.get(fetch.url.clone())
			.send()
			.await?
			.error_for_status()?;

		// Get a reader for the response body.
		let mut stream = response
			.bytes_stream()
			.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error));

		// Stream the body to the temp while computing its hash.
		let mut hasher = Hasher::new(fetch.digest.clone());
		let mut file = tokio::fs::File::create(&temp_path).await?;
		let mut file_writer = tokio::io::BufWriter::new(&mut file);
		while let Some(chunk) = stream.next().await {
			let chunk = chunk?;
			hasher.update(&chunk);
			file_writer.write_all(&chunk).await?;
		}
		file_writer.flush().await?;

		// Verify the hash.
		hasher
			.finalize_and_validate()
			.map_err(|error| anyhow!("Error fetching URL {}: {error}.", fetch.url.clone()))?;

		tracing::trace!(r#"Fetched "{}"."#, fetch.url);

		// Unpack the artifact if requested.
		let artifact = if fetch.unpack {
			let archive_format = ArchiveFormat::for_path(Path::new(fetch.url.path()))
				.with_context(|| {
					anyhow!(r#"Could not determine archive format for "{}"."#, fetch.url)
				})?;
			tracing::trace!(r#"Unpacking the contents of URL "{}"."#, fetch.url);
			let artifact = self.unpack(temp_path, archive_format).await?;
			tracing::trace!(
				r#"Unpacked the contents of URL "{}" to artifact "{}"."#,
				fetch.url,
				artifact,
			);
			artifact
		} else {
			self.checkin(&temp_path).await?
		};

		Ok(artifact)
	}
}

impl State {
	async fn unpack(
		&self,
		archive_temp_path: PathBuf,
		archive_format: ArchiveFormat,
	) -> Result<Hash> {
		// Create a temp to unpack to.
		let unpack_temp_path = self.create_temp_path();

		// Unpack in a blocking task.
		tokio::task::spawn_blocking({
			let unpack_temp_path = unpack_temp_path.clone();
			move || -> Result<_> {
				let archive_file = std::fs::File::open(archive_temp_path)?;
				let archive_reader = std::io::BufReader::new(archive_file);
				match archive_format {
					ArchiveFormat::Tar(compression_format) => {
						let archive_reader: Box<dyn std::io::Read> = match compression_format {
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
							Some(CompressionFormat::Zstd) => {
								Box::new(zstd::Decoder::new(archive_reader)?)
							},
						};
						let mut archive = tar::Archive::new(archive_reader);
						archive.set_preserve_permissions(false);
						archive.set_unpack_xattrs(false);
						archive.unpack(&unpack_temp_path)?;
					},
					ArchiveFormat::Zip => {
						let mut zip = zip::ZipArchive::new(archive_reader)?;
						zip.extract(&unpack_temp_path)?;
					},
				};
				Ok(())
			}
		})
		.await
		.unwrap()?;

		// Check in the temp.
		let artifact = self.checkin(&unpack_temp_path).await?;

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
