use crate::{
	artifact::{self, Artifact},
	blob,
	constants::REFERENCED_ARTIFACTS_DIRECTORY_NAME,
	directory::Directory,
	error::{bail, Context, Result},
	file::File,
	hash, os,
	path::Path,
	reference::Reference,
	symlink::Symlink,
	Instance,
};
use async_recursion::async_recursion;
use futures::future::try_join_all;
use std::{
	fs::Metadata,
	os::unix::prelude::{MetadataExt, PermissionsExt},
};

impl Instance {
	#[async_recursion]
	pub async fn check_in(&self, path: &os::Path) -> Result<artifact::Hash> {
		// Get the metadata for the file system object at the path.
		let metadata = tokio::fs::symlink_metadata(path).await?;

		// Call the appropriate function for the file system object at the path.
		let artifact_hash = if metadata.is_dir() {
			self.check_in_directory(path, &metadata)
				.await
				.with_context(|| {
					let path = path.display();
					format!(r#"Failed to cache the directory at path "{path}"."#)
				})?
		} else if metadata.is_file() {
			self.check_in_file(path, &metadata).await.with_context(|| {
				let path = path.display();
				format!(r#"Failed to check in the file at path "{path}"."#)
			})?
		} else if metadata.is_symlink() {
			self.check_in_symlink(path, &metadata)
				.await
				.with_context(|| {
					let path = path.display();
					format!(r#"Failed to cache the symlink at path "{path}"."#)
				})?
		} else {
			bail!("The path must point to a directory, file, or symlink.")
		};

		Ok(artifact_hash)
	}

	async fn check_in_directory(
		&self,
		path: &os::Path,
		_metadata: &Metadata,
	) -> Result<artifact::Hash> {
		// Read the contents of the directory.
		let permit = self.file_semaphore.acquire().await.unwrap();
		let mut read_dir = tokio::fs::read_dir(path)
			.await
			.context("Failed to read the directory.")?;
		let mut entry_names = Vec::new();
		while let Some(entry) = read_dir.next_entry().await? {
			// Get the entry's file name.
			let file_name = entry
				.file_name()
				.to_str()
				.context("All file names must be valid UTF-8.")?
				.to_owned();

			// Ignore the entry if it is the referenced artifacts directory.
			if file_name == REFERENCED_ARTIFACTS_DIRECTORY_NAME {
				continue;
			}

			// Add the file name to the entry names.
			entry_names.push(file_name);
		}
		drop(read_dir);
		drop(permit);

		// Recurse into the directory's entries.
		let entries = try_join_all(entry_names.into_iter().map(|entry_name| async {
			let entry_path = path.join(&entry_name);
			let artifact_hash = self.check_in(&entry_path).await?;
			Ok::<_, anyhow::Error>((entry_name, artifact_hash))
		}))
		.await?
		.into_iter()
		.collect();

		// Create the artifact.
		let artifact = Artifact::Directory(Directory { entries });

		// Add the artifact.
		let artifact_hash = self.add_artifact(&artifact).await?;

		Ok(artifact_hash)
	}

	async fn check_in_file(&self, path: &os::Path, metadata: &Metadata) -> Result<artifact::Hash> {
		// If there is an artifact tracker whose timestamp matches the file at the path, then return the tracked artifact hash.
		if let Some(artifact_tracker) = self.get_artifact_tracker(path)? {
			let timestamp = std::time::Duration::new(
				metadata.ctime().try_into().unwrap(),
				metadata.ctime_nsec().try_into().unwrap(),
			);
			let tracked_timestamp = std::time::Duration::new(
				artifact_tracker.timestamp_seconds,
				artifact_tracker.timestamp_nanoseconds,
			);
			if tracked_timestamp == timestamp {
				return Ok(artifact_tracker.artifact_hash);
			}
		}

		// Get a file system permit.
		let permit = self.file_semaphore.acquire().await.unwrap();

		// Compute the file's blob hash.
		let mut file = tokio::fs::File::open(path).await?;
		let mut hash_writer = hash::Writer::new();
		tokio::io::copy(&mut file, &mut hash_writer).await?;
		let blob_hash = blob::Hash(hash_writer.finalize());
		drop(file);

		// Copy the file to the temp path.
		let temp_path = self.temp_path();
		let blob_path = self.blob_path(blob_hash);
		tokio::fs::copy(path, &temp_path).await?;

		// Make the temp file readonly.
		let metadata = tokio::fs::metadata(&temp_path).await?;
		let mut permissions = metadata.permissions();
		permissions.set_readonly(true);
		tokio::fs::set_permissions(&temp_path, permissions).await?;

		// Move the file to the blobs directory.
		tokio::fs::rename(&temp_path, &blob_path).await?;

		// Drop the file system permit.
		drop(permit);

		// Determine if the file is executable.
		let executable = (metadata.permissions().mode() & 0o111) != 0;

		// Create the artifact.
		let artifact = Artifact::File(File {
			blob_hash,
			executable,
		});

		// Add the artifact.
		let artifact_hash = self.add_artifact(&artifact).await?;

		// Add the artifact tracker.
		let timestamp_seconds = metadata.ctime().try_into().unwrap();
		let timestamp_nanoseconds = metadata.ctime_nsec().try_into().unwrap();
		let entry = artifact::Tracker {
			artifact_hash,
			timestamp_seconds,
			timestamp_nanoseconds,
		};
		self.add_artifact_tracker(path, &entry)?;

		Ok(artifact_hash)
	}

	async fn check_in_symlink(
		&self,
		path: &os::Path,
		_metadata: &Metadata,
	) -> Result<artifact::Hash> {
		// Read the symlink.
		let permit = self.file_semaphore.acquire().await.unwrap();
		let target = tokio::fs::read_link(path).await.with_context(|| {
			format!(
				r#"Failed to read the symlink at path "{}"."#,
				path.display()
			)
		})?;
		drop(permit);

		// Create the artifact. A symlink is a reference if the result of canonicalizing its path's parent joined with its target points into the checkouts directory.
		let target_in_checkouts_path = tokio::fs::canonicalize(&path.join("..").join(&target))
			.await
			.ok()
			.and_then(|canonicalized_target| {
				let target_in_checkouts_path = canonicalized_target
					.strip_prefix(&self.checkouts_path())
					.ok()?;
				Some(target_in_checkouts_path.to_owned())
			});
		let artifact = if let Some(target_in_checkouts_path) = target_in_checkouts_path {
			// Convert the target to a path.
			let target: Path = target_in_checkouts_path
				.as_os_str()
				.to_str()
				.context("The symlink target was not valid UTF-8.")?
				.parse()
				.context("The target is not a valid path.")?;

			// Get the path components.
			let mut components = target.components.iter().peekable();

			// Parse the hash from the first component.
			let artifact_hash: artifact::Hash = components
				.next()
				.context("Invalid symlink.")?
				.as_str()
				.parse()
				.context("Failed to parse the path component as a hash.")?;

			// Collect the remaining components to get the path within the referenced artifact.
			let path = if components.peek().is_some() {
				Some(components.cloned().collect())
			} else {
				None
			};

			Artifact::Reference(Reference {
				artifact_hash,
				path,
			})
		} else {
			// Convert the target to a string.
			let target = target
				.into_os_string()
				.into_string()
				.ok()
				.context("The symlink target was not valid UTF-8.")?;

			Artifact::Symlink(Symlink { target })
		};

		// Add the artifact.
		let artifact_hash = self.add_artifact(&artifact).await?;

		Ok(artifact_hash)
	}
}
