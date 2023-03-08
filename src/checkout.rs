use crate::{
	artifact::{self, Artifact},
	constants::REFERENCED_ARTIFACTS_DIRECTORY_NAME,
	directory::Directory,
	file::File,
	os,
	reference::Reference,
	symlink::Symlink,
	Instance,
};
use anyhow::{Context, Result};
use async_recursion::async_recursion;
use futures::future::try_join_all;
use std::os::unix::prelude::PermissionsExt;

impl Instance {
	pub async fn check_out_internal(&self, artifact_hash: artifact::Hash) -> Result<os::PathBuf> {
		// Compute the checkout's path in the checkouts directory.
		let path = self.checkouts_path().join(artifact_hash.to_string());

		// Create a temp path.
		let temp_path = self.temp_path();

		// Perform the checkout to the temp path.
		self.check_out_internal_inner(artifact_hash, &temp_path)
			.await?;

		// Make the file system object writeable.
		let metadata = tokio::fs::metadata(&temp_path).await?;
		let mut permissions = metadata.permissions();
		permissions.set_readonly(false);
		tokio::fs::set_permissions(&temp_path, permissions).await?;

		// Move the checkout from the temp path to the path in the checkouts directory.
		match tokio::fs::rename(&temp_path, &path).await {
			Ok(()) => Ok(()),

			// If the error is ENOTEMPTY or EEXIST, then we can ignore it because there is already an artifact checkout present.
			Err(error) if matches!(error.raw_os_error(), Some(libc::ENOTEMPTY | libc::EEXIST)) => {
				Ok(())
			},

			Err(error) => Err(error),
		}
		.context("Failed to move the checkout to the checkout path.")?;

		// Make the file system object readonly.
		let metadata = tokio::fs::metadata(&path).await?;
		let mut permissions = metadata.permissions();
		permissions.set_readonly(true);
		tokio::fs::set_permissions(&path, permissions).await?;

		// Clear the file system object's timestamps.
		tokio::task::spawn_blocking({
			let path = path.clone();
			move || {
				let epoch = filetime::FileTime::from_unix_time(0, 0);
				filetime::set_symlink_file_times(path, epoch, epoch)
					.context("Failed to set the file system object's timestamps.")?;
				Ok::<_, anyhow::Error>(())
			}
		})
		.await
		.unwrap()?;

		Ok(path)
	}

	#[async_recursion]
	pub async fn check_out_internal_inner(
		&self,
		artifact_hash: artifact::Hash,
		path: &os::Path,
	) -> Result<()> {
		// Get the artifact.
		let artifact = self.get_artifact_local(artifact_hash)?;

		match artifact {
			Artifact::Directory(directory) => {
				// Create the directory.
				tokio::fs::create_dir(path).await?;

				// Recurse into the entries.
				try_join_all(directory.entries.into_iter().map(
					|(entry_name, entry_hash)| async move {
						let entry_path = path.join(&entry_name);
						self.check_out_internal_inner(entry_hash, &entry_path)
							.await?;
						Ok::<_, anyhow::Error>(())
					},
				))
				.await?;
			},

			Artifact::File(file) => {
				// Copy the blob to the path.
				self.copy_blob_to_path(file.blob_hash, path)
					.await
					.context("Failed to copy the blob.")?;

				// Make the file executable if necessary.
				if file.executable {
					let metadata = tokio::fs::metadata(&path).await?;
					let mut permissions = metadata.permissions();
					permissions.set_mode(0o755);
					tokio::fs::set_permissions(&path, permissions).await?;
				}
			},

			Artifact::Symlink(symlink) => {
				// Create the symlink.
				tokio::fs::symlink(symlink.target, path).await?;
			},

			Artifact::Reference(reference) => {
				// Check out the referenced artifact.
				let referenced_artifact_checkout_path = self
					.check_out_internal(reference.artifact_hash)
					.await
					.context("Failed to check out the referenced artifact.")?;

				// Compute the referenced path.
				let mut referenced_path = referenced_artifact_checkout_path;
				if let Some(reference_path) = reference.path {
					referenced_path.push(reference_path.to_string());
				}

				// Compute the symlink target by taking the diff of the path's parent and the referenced path.
				let parent_path = path
					.parent()
					.context("Expected the path to have a parent.")?;
				let target = pathdiff::diff_paths(&referenced_path, parent_path)
					.context("Could not resolve the symlink target relative to the path.")?;

				// Create the symlink.
				tokio::fs::symlink(target, path)
					.await
					.context("Failed to write the symlink for the reference.")?;
			},
		};

		// Make the file system object readonly.
		let metadata = tokio::fs::metadata(&path).await?;
		let mut permissions = metadata.permissions();
		permissions.set_readonly(true);
		tokio::fs::set_permissions(&path, permissions).await?;

		// Clear the file system object's timestamps.
		tokio::task::spawn_blocking({
			let path = path.to_owned();
			move || {
				let epoch = filetime::FileTime::from_unix_time(0, 0);
				filetime::set_symlink_file_times(path, epoch, epoch)
					.context("Failed to set the file system object's timestamps.")?;
				Ok::<_, anyhow::Error>(())
			}
		})
		.await
		.unwrap()?;

		Ok(())
	}
}

impl Instance {
	pub async fn check_out_external(
		&self,
		artifact_hash: artifact::Hash,
		path: &os::Path,
	) -> Result<()> {
		// Check in an existing artifact at the path.
		let existing_artifact_hash = if os::fs::exists(path).await? {
			Some(self.check_in(path).await?)
		} else {
			None
		};

		// Check out the artifact recursively.
		self.check_out_external_inner(path, existing_artifact_hash, artifact_hash, path)
			.await?;

		Ok(())
	}

	async fn check_out_external_inner(
		&self,
		root_path: &os::Path,
		existing_artifact_hash: Option<artifact::Hash>,
		artifact_hash: artifact::Hash,
		path: &os::Path,
	) -> Result<()> {
		// If the artifact hash matches the existing artifact hash, then return.
		if existing_artifact_hash.map_or(false, |existing_artifact_hash| {
			existing_artifact_hash == artifact_hash
		}) {
			return Ok(());
		}

		// Get the artifact.
		let artifact = self.get_artifact_local(artifact_hash)?;

		// Call the appropriate function for the artifact's type.
		match artifact {
			Artifact::Directory(directory) => {
				self.check_out_directory(
					root_path,
					existing_artifact_hash,
					artifact_hash,
					directory,
					path,
				)
				.await
				.with_context(|| {
					let path = path.display();
					format!(r#"Failed to check out directory "{artifact_hash}" to "{path}"."#)
				})?;
			},

			Artifact::File(file) => {
				self.check_out_file(root_path, existing_artifact_hash, artifact_hash, file, path)
					.await
					.with_context(|| {
						let path = path.display();
						format!(r#"Failed to check out file "{artifact_hash}" to "{path}"."#)
					})?;
			},

			Artifact::Symlink(symlink) => {
				self.check_out_symlink(
					root_path,
					existing_artifact_hash,
					artifact_hash,
					symlink,
					path,
				)
				.await
				.with_context(|| {
					let path = path.display();
					format!(r#"Failed to check out symlink "{artifact_hash}" to "{path}"."#)
				})?;
			},

			Artifact::Reference(reference) => {
				self.check_out_reference(
					root_path,
					existing_artifact_hash,
					artifact_hash,
					reference,
					path,
				)
				.await
				.with_context(|| {
					let path = path.display();
					format!(r#"Failed to check out reference "{artifact_hash}" to "{path}"."#)
				})?;
			},
		}

		Ok(())
	}

	#[async_recursion]
	async fn check_out_directory(
		&self,
		root_path: &os::Path,
		existing_artifact_hash: Option<artifact::Hash>,
		_artifact_hash: artifact::Hash,
		directory: Directory,
		path: &os::Path,
	) -> Result<()> {
		// Get the artifact for an existing file system object at the path.
		let existing_artifact = if let Some(existing_artifact_hash) = existing_artifact_hash {
			Some(self.get_artifact_local(existing_artifact_hash)?)
		} else {
			None
		};

		// Handle an existing artifact at the path.
		match &existing_artifact {
			// If there is already a directory, then remove any extraneous entries.
			Some(Artifact::Directory(existing_directory)) => {
				try_join_all(existing_directory.entries.keys().map(|entry_name| {
					let directory = &directory;
					async move {
						if !directory.entries.contains_key(entry_name) {
							let entry_path = path.join(entry_name);
							os::fs::rmrf(&entry_path, None).await?;
						}
						Ok::<_, anyhow::Error>(())
					}
				}))
				.await?;
			},

			// If there is an existing artifact at the path and it is not a directory, then remove it, create a directory, and continue.
			Some(_) => {
				os::fs::rmrf(path, None).await?;
				tokio::fs::create_dir_all(path).await?;
			},

			// If there is no artifact at this path, then create a directory.
			None => {
				tokio::fs::create_dir_all(path).await?;
			},
		};

		// Recurse into the entries.
		try_join_all(
			directory
				.entries
				.into_iter()
				.map(|(entry_name, entry_hash)| {
					let existing_artifact = &existing_artifact;
					async move {
						// Retrieve an existing artifact.
						let existing_artifact_hash = match existing_artifact {
							Some(Artifact::Directory(existing_directory)) => {
								existing_directory.entries.get(&entry_name).copied()
							},
							_ => None,
						};

						// Recurse.
						let entry_path = path.join(&entry_name);
						self.check_out_external_inner(
							root_path,
							existing_artifact_hash,
							entry_hash,
							&entry_path,
						)
						.await?;

						Ok::<_, anyhow::Error>(())
					}
				}),
		)
		.await?;

		Ok(())
	}

	async fn check_out_file(
		&self,
		_root_path: &os::Path,
		existing_artifact_hash: Option<artifact::Hash>,
		_artifact_hash: artifact::Hash,
		file: File,
		path: &os::Path,
	) -> Result<()> {
		// Get the artifact for an existing file system object at the path.
		let existing_artifact = if let Some(existing_artifact_hash) = existing_artifact_hash {
			Some(self.get_artifact_local(existing_artifact_hash)?)
		} else {
			None
		};

		// Handle an existing artifact at the path.
		match &existing_artifact {
			// If there is an existing file system object at the path, then remove it and continue.
			Some(_) => {
				os::fs::rmrf(path, None).await?;
			},

			// If there is no file system object at this path, then continue.
			None => {},
		};

		// Copy the blob to the path.
		self.copy_blob_to_path(file.blob_hash, path)
			.await
			.context("Failed to copy the blob.")?;

		// Make the file executable if necessary.
		if file.executable {
			let metadata = tokio::fs::metadata(&path).await?;
			let mut permissions = metadata.permissions();
			permissions.set_mode(0o755);
			tokio::fs::set_permissions(&path, permissions).await?;
		}

		Ok(())
	}

	async fn check_out_symlink(
		&self,
		_root_path: &os::Path,
		existing_artifact_hash: Option<artifact::Hash>,
		_artifact_hash: artifact::Hash,
		symlink: Symlink,
		path: &os::Path,
	) -> Result<()> {
		// Get the artifact for an existing file system object at the path.
		let existing_artifact = if let Some(existing_artifact_hash) = existing_artifact_hash {
			Some(self.get_artifact_local(existing_artifact_hash)?)
		} else {
			None
		};

		// Handle an existing artifact at the path.
		match &existing_artifact {
			// If there is an existing file system object at the path, then remove it and continue.
			Some(_) => {
				os::fs::rmrf(path, None).await?;
			},

			// If there is no file system object at this path, then continue.
			None => {},
		};

		// Create the symlink.
		tokio::fs::symlink(symlink.target, path).await?;

		Ok(())
	}

	#[async_recursion]
	async fn check_out_reference(
		&self,
		root_path: &os::Path,
		existing_artifact_hash: Option<artifact::Hash>,
		artifact_hash: artifact::Hash,
		reference: Reference,
		path: &os::Path,
	) -> Result<()> {
		// Get the artifact for an existing file system object at the path.
		let existing_artifact = if let Some(existing_artifact_hash) = existing_artifact_hash {
			Some(self.get_artifact_local(existing_artifact_hash)?)
		} else {
			None
		};

		// Handle an existing artifact at the path.
		match &existing_artifact {
			// If there is an existing artifact at the path, then remove it and continue.
			Some(_) => {
				os::fs::rmrf(path, None).await?;
			},

			// If there is no artifact at this path, then continue.
			None => {},
		};

		// Create the referenced artifacts path.
		let referenced_artifacts_path = root_path.join(REFERENCED_ARTIFACTS_DIRECTORY_NAME);

		// Get the referenced artifact checkout path.
		let referenced_artifact_checkout_path =
			referenced_artifacts_path.join(artifact_hash.to_string());

		// Check out the referenced artifact if necessary.
		if !os::fs::exists(&referenced_artifact_checkout_path).await? {
			// Create the referenced artifact checkout path's parent directory if necessary.
			tokio::fs::create_dir_all(&referenced_artifact_checkout_path).await?;

			// Perform the checkout.
			self.check_out_external_inner(
				root_path,
				None,
				reference.artifact_hash,
				&referenced_artifact_checkout_path,
			)
			.await
			.context("Failed to check out the referenced artifact.")?;
		}

		// Compute the referenced path.
		let mut referenced_path = referenced_artifact_checkout_path;
		if let Some(reference_path) = reference.path {
			referenced_path.push(reference_path.to_string());
		}

		// Compute the symlink target by taking the diff of the path's parent and the referenced path.
		let parent_path = path
			.parent()
			.context("Expected the path to have a parent.")?;
		let target = pathdiff::diff_paths(&referenced_path, parent_path)
			.context("Could not resolve the symlink target relative to the path.")?;

		// Create the symlink.
		tokio::fs::symlink(target, path)
			.await
			.context("Failed to write the symlink for the reference.")?;

		Ok(())
	}
}
