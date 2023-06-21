use crate::{
	artifact::{Artifact},
	blob::{self, Blob},
	directory::Directory,
	error::{return_error, Error, Result, WrapErr},
	file::File,
	hash,
	instance::Instance,
	symlink::Symlink,
	temp::Temp,
	template::Template,
};
use async_recursion::async_recursion;
use futures::future::try_join_all;
use std::{
	fs::Metadata,
	os::unix::prelude::PermissionsExt,
	path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default)]
pub struct Options {
	pub artifacts_paths: Vec<PathBuf>,
}

impl Artifact {
	pub async fn check_in(tg: &Instance, path: &Path) -> Result<Self> {
		Self::check_in_with_options(tg, path, &Options::default()).await
	}

	#[async_recursion]
	pub async fn check_in_with_options(
		tg: &Instance,
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
			Self::check_in_directory(tg, path, &metadata, options)
				.await
				.wrap_err_with(|| {
					let path = path.display();
					format!(r#"Failed to cache the directory at path "{path}"."#)
				})?
		} else if metadata.is_file() {
			Self::check_in_file(tg, path, &metadata, options)
				.await
				.wrap_err_with(|| {
					let path = path.display();
					format!(r#"Failed to check in the file at path "{path}"."#)
				})?
		} else if metadata.is_symlink() {
			Self::check_in_symlink(tg, path, &metadata, options)
				.await
				.wrap_err_with(|| {
					let path = path.display();
					format!(r#"Failed to cache the symlink at path "{path}"."#)
				})?
		} else {
			return_error!("The path must point to a directory, file, or symlink.")
		};

		Ok(artifact)
	}

	async fn check_in_directory(
		tg: &Instance,
		path: &Path,
		_metadata: &Metadata,
		options: &Options,
	) -> Result<Self> {
		// Read the contents of the directory.
		let permit = tg.file_descriptor_semaphore.acquire().await;
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
		drop(read_dir);
		drop(permit);

		// Recurse into the directory's entries.
		let entries = try_join_all(names.into_iter().map(|name| async {
			let path = path.join(&name);
			let artifact = Self::check_in_with_options(tg, &path, options).await?;
			Ok::<_, Error>((name, artifact))
		}))
		.await?
		.into_iter()
		.collect();

		// Create the directory.
		let directory = Directory::new(tg, &entries)?;

		Ok(directory.into())
	}

	async fn check_in_file(
		tg: &Instance,
		path: &Path,
		metadata: &Metadata,
		_options: &Options,
	) -> Result<Self> {
		// // If there is an artifact tracker whose timestamp matches the file at the path, then return the tracked artifact hash.
		// if let Some(artifact_tracker) = tg.get_artifact_tracker(path)? {
		// 	let timestamp = std::time::Duration::new(
		// 		metadata.ctime().try_into().unwrap(),
		// 		metadata.ctime_nsec().try_into().unwrap(),
		// 	);
		// 	let tracked_timestamp = std::time::Duration::new(
		// 		artifact_tracker.timestamp_seconds,
		// 		artifact_tracker.timestamp_nanoseconds,
		// 	);
		// 	if tracked_timestamp == timestamp {
		// 		return Ok(artifact_tracker.artifact_hash);
		// 	}
		// }

		// Compute the file's hash.
		let permit = tg.file_descriptor_semaphore.acquire().await;
		let mut file = tokio::fs::File::open(path).await?;
		let mut hash_writer = hash::Writer::new();
		tokio::io::copy(&mut file, &mut hash_writer).await?;
		let blob_hash = blob::Hash(hash_writer.finalize());
		drop(file);

		// Copy the file to the temp path.
		let temp = Temp::new(tg);
		let blob_path = tg.blob_path(blob_hash);
		tokio::fs::copy(path, temp.path()).await?;
		drop(permit);

		// Set the permissions.
		let permissions = std::fs::Permissions::from_mode(0o444);
		tokio::fs::set_permissions(temp.path(), permissions).await?;

		// Move the file to the blobs directory.
		tokio::fs::rename(temp.path(), &blob_path).await?;

		// Create the blob.
		let blob = Blob::from_hash(blob_hash);

		// Determine if the file is executable.
		let executable = (metadata.permissions().mode() & 0o111) != 0;

		// The file has no references.
		let references = [];

		// Create the file.
		let file = File::new(tg, blob, executable, &references)?;

		// // Add the artifact tracker.
		// let timestamp_seconds = metadata.ctime().try_into().unwrap();
		// let timestamp_nanoseconds = metadata.ctime_nsec().try_into().unwrap();
		// let entry = artifact::Tracker {
		// 	artifact_hash,
		// 	timestamp_seconds,
		// 	timestamp_nanoseconds,
		// };
		// tg.add_artifact_tracker(path, &entry)?;

		Ok(file.into())
	}

	async fn check_in_symlink(
		tg: &Instance,
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
		let target = Template::unrender(tg, &options.artifacts_paths, target).await?;

		// Create the symlink.
		let symlink = Symlink::new(tg, target)?;

		Ok(symlink.into())
	}
}
