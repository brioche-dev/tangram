use crate::{
	artifact::{self, Artifact},
	blob,
	directory::Directory,
	error::{return_error, Error, Result, WrapErr},
	file::File,
	hash,
	symlink::Symlink,
	temp::Temp,
	util::fs,
	Instance,
};
use async_recursion::async_recursion;
use futures::future::try_join_all;
use std::{
	fs::Metadata,
	os::unix::prelude::{MetadataExt, PermissionsExt},
};

#[derive(serde::Deserialize)]
struct Attributes {
	references: Vec<artifact::Hash>,
}

impl Instance {
	#[async_recursion]
	pub async fn check_in(&self, path: &fs::Path) -> Result<artifact::Hash> {
		// Get the metadata for the file system object at the path.
		let metadata = tokio::fs::symlink_metadata(path).await.wrap_err_with(|| {
			let path = path.display();
			format!(r#"Failed to get the metadata for the path "{path}"."#)
		})?;

		// Call the appropriate function for the file system object at the path.
		let artifact_hash = if metadata.is_dir() {
			self.check_in_directory(path, &metadata)
				.await
				.wrap_err_with(|| {
					let path = path.display();
					format!(r#"Failed to cache the directory at path "{path}"."#)
				})?
		} else if metadata.is_file() {
			self.check_in_file(path, &metadata)
				.await
				.wrap_err_with(|| {
					let path = path.display();
					format!(r#"Failed to check in the file at path "{path}"."#)
				})?
		} else if metadata.is_symlink() {
			self.check_in_symlink(path, &metadata)
				.await
				.wrap_err_with(|| {
					let path = path.display();
					format!(r#"Failed to cache the symlink at path "{path}"."#)
				})?
		} else {
			return_error!("The path must point to a directory, file, or symlink.")
		};

		Ok(artifact_hash)
	}

	async fn check_in_directory(
		&self,
		path: &fs::Path,
		_metadata: &Metadata,
	) -> Result<artifact::Hash> {
		// Read the contents of the directory.
		let permit = self.file_semaphore.acquire().await.map_err(Error::other)?;
		let mut read_dir = tokio::fs::read_dir(path)
			.await
			.wrap_err("Failed to read the directory.")?;
		let mut entry_names = Vec::new();
		while let Some(entry) = read_dir.next_entry().await? {
			// Get the entry's file name.
			let file_name = entry
				.file_name()
				.to_str()
				.wrap_err("All file names must be valid UTF-8.")?
				.to_owned();

			// Add the file name to the entry names.
			entry_names.push(file_name);
		}
		drop(read_dir);
		drop(permit);

		// Recurse into the directory's entries.
		let entries = try_join_all(entry_names.into_iter().map(|entry_name| async {
			let entry_path = path.join(&entry_name);
			let artifact_hash = self.check_in(&entry_path).await?;
			Ok::<_, Error>((entry_name, artifact_hash))
		}))
		.await?
		.into_iter()
		.collect();

		// Create the artifact.
		let artifact = Artifact::Directory(Directory::new(entries));

		// Add the artifact.
		let artifact_hash = self.add_artifact(&artifact).await?;

		Ok(artifact_hash)
	}

	async fn check_in_file(&self, path: &fs::Path, metadata: &Metadata) -> Result<artifact::Hash> {
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

		// Compute the file's blob hash.
		let permit = self.file_semaphore.acquire().await.map_err(Error::other)?;
		let mut file = tokio::fs::File::open(path).await?;
		let mut hash_writer = hash::Writer::new();
		tokio::io::copy(&mut file, &mut hash_writer).await?;
		let blob_hash = blob::Hash(hash_writer.finalize());
		drop(file);
		drop(permit);

		// Copy the file to the temp path.
		let temp = Temp::new(self);
		let blob_path = self.blob_path(blob_hash);
		tokio::fs::copy(path, temp.path()).await?;

		// Set the permissions.
		let permissions = std::fs::Permissions::from_mode(0o644);
		tokio::fs::set_permissions(temp.path(), permissions).await?;

		// Move the file to the blobs directory.
		tokio::fs::rename(temp.path(), &blob_path).await?;

		// Determine if the file is executable.
		let executable = (metadata.permissions().mode() & 0o111) != 0;

		// Read the file's references from its xattrs.
		let attributes: Option<Attributes> = xattr::get(path, "user.tangram")
			.ok()
			.flatten()
			.and_then(|attributes| serde_json::from_slice(&attributes).ok());
		let references = attributes.map_or_else(Vec::new, |a| a.references);

		// Create the artifact.
		let artifact = Artifact::File(File::new(blob_hash, executable, references));

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
		path: &fs::Path,
		_metadata: &Metadata,
	) -> Result<artifact::Hash> {
		// Read the target from the symlink.
		let permit = self.file_semaphore.acquire().await.map_err(Error::other)?;
		let target = tokio::fs::read_link(path).await.wrap_err_with(|| {
			format!(
				r#"Failed to read the symlink at path "{}"."#,
				path.display()
			)
		})?;
		drop(permit);

		// Unrender the target.
		let artifacts_path = self.artifacts_path();
		let target = target
			.to_str()
			.wrap_err("The symlink target must be valid UTF-8.")?;
		let target = self.unrender(&artifacts_path, target).await?;

		// Create the artifact.
		let artifact = Artifact::Symlink(Symlink { target });

		// Add the artifact.
		let artifact_hash = self
			.add_artifact(&artifact)
			.await
			.wrap_err("Failed to add the artifact.")?;

		Ok(artifact_hash)
	}
}
