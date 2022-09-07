use crate::{
	artifact::Artifact,
	expression::{self},
	hash::Hasher,
	object::{BlobHash, File, Object},
	server::Server,
	value::Value,
};
use anyhow::{anyhow, bail, Result};
use futures::{StreamExt, TryStreamExt};
use std::{path::Path, sync::Arc};
use tokio::io::AsyncWriteExt;

impl Server {
	pub async fn evaluate_fetch(self: &Arc<Self>, fetch: &expression::Fetch) -> Result<Value> {
		tracing::trace!(r#"Fetching "{}"."#, fetch.url);

		// Retrieve the artifact if it has been downloaded.
		let artifact = if let Some(hash) = fetch.hash {
			if self.get_blob(BlobHash(hash)).await?.is_some() {
				let object = Object::File(File {
					blob_hash: BlobHash(hash),
					executable: false,
				});
				Some(Artifact::new(object.hash()))
			} else {
				None
			}
		} else {
			None
		};

		// Download the the URL if it has not been downloaded yet.
		let artifact = if let Some(artifact) = artifact {
			artifact
		} else {
			// Create a temp.
			let temp = self.create_temp().await?;
			let temp_path = self.temp_path(&temp);

			// Perform the request and get a reader for the body.
			let response = self.http_client.get(fetch.url.clone()).send().await?;
			let response = response.error_for_status()?;
			let mut stream = response
				.bytes_stream()
				.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error));

			// Stream the body to the temp while computing its hash.
			let mut hasher = Hasher::new();
			let mut file = tokio::fs::File::create(&temp_path).await?;
			let mut file_writer = tokio::io::BufWriter::new(&mut file);
			while let Some(chunk) = stream.next().await {
				let chunk = chunk?;
				hasher.write_all(&chunk).await?;
				file_writer.write_all(&chunk).await?;
			}
			let hash = hasher.finalize();
			file_writer.flush().await?;

			// Verify the hash.
			match (fetch.hash, hash) {
				(None, _) => bail!("Missing hash!\nReceived: {}\n", hash),
				(Some(fetch_hash), hash) if fetch_hash != hash => {
					bail!(
						"Hash mismatch in fetch!\nExpected: {}\nReceived: {}\n",
						fetch_hash,
						hash,
					);
				},
				_ => {},
			};

			// Checkin the temp.
			self.checkin_temp(temp).await?
		};

		tracing::trace!(r#"Fetched "{}" to artifact "{}"."#, fetch.url, artifact);

		// Unpack the artifact if requested.
		let artifact = if fetch.unpack {
			let archive_format =
				ArchiveFormat::for_path(Path::new(fetch.url.path())).ok_or_else(|| {
					anyhow!(r#"Could not determine archive format for "{}"."#, fetch.url)
				})?;
			tracing::trace!(r#"Unpacking contents of URL "{}"."#, fetch.url);
			let artifact = self.unpack(artifact, archive_format).await?;
			tracing::trace!(
				r#"Unpacked contents of URL "{}" to artifact "{}"."#,
				fetch.url,
				artifact
			);
			artifact
		} else {
			artifact
		};

		// Create the value.
		let value = Value::Artifact(artifact);

		Ok(value)
	}

	async fn unpack(
		self: &Arc<Self>,
		artifact: Artifact,
		archive_format: ArchiveFormat,
	) -> Result<Artifact> {
		// Checkout the archive.
		let archive_fragment = self.create_fragment(artifact).await?;
		let archive_fragment_path = self.fragment_path(&archive_fragment);

		// Create a temp to unpack to.
		let unpack_temp = self.create_temp().await?;
		let unpack_temp_path = self.temp_path(&unpack_temp);

		// Unpack in a blocking task.
		tokio::task::spawn_blocking(move || -> Result<_> {
			let archive_file = std::fs::File::open(archive_fragment_path)?;
			let archive_reader = std::io::BufReader::new(archive_file);
			match archive_format {
				ArchiveFormat::Tar => {
					let mut archive = tar::Archive::new(archive_reader);
					archive.set_preserve_permissions(false);
					archive.set_unpack_xattrs(false);
					archive.unpack(&unpack_temp_path)?;
				},
				ArchiveFormat::TarBz2 => {
					let mut archive =
						tar::Archive::new(bzip2::read::BzDecoder::new(archive_reader));
					archive.set_preserve_permissions(false);
					archive.set_unpack_xattrs(false);
					archive.unpack(&unpack_temp_path)?;
				},
				ArchiveFormat::TarGz => {
					let mut archive =
						tar::Archive::new(flate2::read::GzDecoder::new(archive_reader));
					archive.set_preserve_permissions(false);
					archive.set_unpack_xattrs(false);
					archive.unpack(&unpack_temp_path)?;
				},
				ArchiveFormat::TarXz => {
					let mut archive = tar::Archive::new(xz::read::XzDecoder::new(archive_reader));
					archive.set_preserve_permissions(false);
					archive.set_unpack_xattrs(false);
					archive.unpack(&unpack_temp_path)?;
				},
				ArchiveFormat::TarZstd => {
					let mut archive = tar::Archive::new(zstd::Decoder::new(archive_reader)?);
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
		})
		.await
		.unwrap()?;

		// Checkin the temp.
		let artifact = self.checkin_temp(unpack_temp).await?;

		Ok(artifact)
	}
}

enum ArchiveFormat {
	TarBz2,
	TarGz,
	TarXz,
	TarZstd,
	Tar,
	Zip,
}

impl ArchiveFormat {
	#[allow(clippy::case_sensitive_file_extension_comparisons)]
	pub fn for_path(path: &Path) -> Option<ArchiveFormat> {
		let path = path.to_str().unwrap();
		if path.ends_with(".tar.bz2") || path.ends_with(".tbz2") {
			Some(ArchiveFormat::TarBz2)
		} else if path.ends_with(".tar.gz") || path.ends_with(".tgz") {
			Some(ArchiveFormat::TarGz)
		} else if path.ends_with(".tar.xz") || path.ends_with(".txz") {
			Some(ArchiveFormat::TarXz)
		} else if path.ends_with(".tar.zstd")
			|| path.ends_with(".tzstd")
			|| path.ends_with(".tar.zst")
			|| path.ends_with(".tzst")
		{
			Some(ArchiveFormat::TarZstd)
		} else if path.ends_with(".tar") {
			Some(ArchiveFormat::Tar)
		} else if path.ends_with(".zip") {
			Some(ArchiveFormat::Zip)
		} else {
			None
		}
	}
}
