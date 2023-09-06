use crate::{self as tg, return_error, Client, Error, Result, WrapErr};
use async_recursion::async_recursion;
use futures::{stream::FuturesUnordered, TryStreamExt};
use std::{
	fs::Metadata,
	os::unix::prelude::PermissionsExt,
	path::{Path, PathBuf},
};

#[derive(serde::Deserialize)]
struct Attributes {
	references: Vec<tg::artifact::Id>,
}

#[derive(Clone, Debug, Default)]
pub struct Options {
	pub artifacts_paths: Vec<PathBuf>,
}

impl crate::Artifact {
	pub async fn check_in(client: &Client, path: &Path) -> Result<Self> {
		Self::check_in_with_options(client, path, &Options::default()).await
	}

	#[async_recursion]
	pub async fn check_in_with_options(
		client: &Client,
		path: &Path,
		options: &Options,
	) -> Result<Self> {
		// Get the metadata for the file system object at the path.
		let metadata = tokio::fs::symlink_metadata(path).await.wrap_err_with(|| {
			let path = path.display();
			format!(r#"Failed to get the metadata for the path "{path}"."#)
		})?;

		// Call the appropriate function for the file system object at the path.
		let artifact = if metadata.is_dir() {
			Self::check_in_directory(client, path, &metadata, options)
				.await
				.wrap_err_with(|| {
					let path = path.display();
					format!(r#"Failed to check in the directory at path "{path}"."#)
				})?
		} else if metadata.is_file() {
			Self::check_in_file(client, path, &metadata, options)
				.await
				.wrap_err_with(|| {
					let path = path.display();
					format!(r#"Failed to check in the file at path "{path}"."#)
				})?
		} else if metadata.is_symlink() {
			Self::check_in_symlink(client, path, &metadata, options)
				.await
				.wrap_err_with(|| {
					let path = path.display();
					format!(r#"Failed to check in the symlink at path "{path}"."#)
				})?
		} else {
			return_error!("The path must point to a directory, file, or symlink.")
		};

		Ok(artifact)
	}

	async fn check_in_directory(
		client: &Client,
		path: &Path,
		_metadata: &Metadata,
		options: &Options,
	) -> Result<Self> {
		// Read the contents of the directory.
		let names = {
			let _permit = client.file_descriptor_semaphore().acquire().await;
			let mut read_dir = tokio::fs::read_dir(path)
				.await
				.wrap_err("Failed to read the directory.")?;
			let mut names = Vec::new();
			while let Some(entry) = read_dir.next_entry().await? {
				let name = entry
					.file_name()
					.to_str()
					.wrap_err("All file names must be valid UTF-8.")?
					.to_owned();
				names.push(name);
			}
			names
		};

		// Recurse into the directory's entries.
		let entries = names
			.into_iter()
			.map(|name| async {
				let path = path.join(&name);
				let artifact = Self::check_in_with_options(client, &path, options).await?;
				Ok::<_, Error>((name, artifact))
			})
			.collect::<FuturesUnordered<_>>()
			.try_collect()
			.await?;

		// Create the directory.
		let directory = tg::Directory::new(entries);

		Ok(directory.into())
	}

	async fn check_in_file(
		client: &Client,
		path: &Path,
		metadata: &Metadata,
		_options: &Options,
	) -> Result<Self> {
		// Create the blob.
		let permit = client.file_descriptor_semaphore().acquire().await;
		let file = tokio::fs::File::open(path)
			.await
			.wrap_err("Failed to open the file.")?;
		let contents = tg::Blob::with_reader(client, file)
			.await
			.wrap_err("Failed to create the contents.")?;
		drop(permit);

		// Determine if the file is executable.
		let executable = (metadata.permissions().mode() & 0o111) != 0;

		// Read the file's references from its xattrs.
		let attributes: Option<Attributes> = xattr::get(path, "user.tangram")
			.ok()
			.flatten()
			.and_then(|attributes| serde_json::from_slice(&attributes).ok());
		let references = attributes
			.map(|attributes| attributes.references)
			.unwrap_or_default()
			.into_iter()
			.map(tg::Artifact::with_id)
			.collect();

		// Create the file.
		let file = tg::File::new(contents, executable, references);

		Ok(file.into())
	}

	async fn check_in_symlink(
		_client: &Client,
		path: &Path,
		_metadata: &Metadata,
		options: &Options,
	) -> Result<Self> {
		// Read the target from the symlink.
		let target = tokio::fs::read_link(path).await.wrap_err_with(|| {
			format!(
				r#"Failed to read the symlink at path "{}"."#,
				path.display(),
			)
		})?;

		// Unrender the target.
		let target = target
			.to_str()
			.wrap_err("The symlink target must be valid UTF-8.")?;
		let target = tg::Template::unrender(&options.artifacts_paths, target)?;

		// Create the symlink.
		let symlink = tg::Symlink::new(target);

		Ok(symlink.into())
	}
}
