use crate::{
	artifact::{self, Artifact},
	directory::Directory,
	file::File,
	os,
	reference::Reference,
	symlink::Symlink,
	Instance,
};
use anyhow::{Context, Result};
use async_recursion::async_recursion;
use futures::{future::try_join_all, Future, FutureExt};
use std::{os::unix::prelude::PermissionsExt, pin::Pin, sync::Arc};

pub type ReferenceHandlerFn =
	dyn Fn(&Reference, &os::Path) -> Pin<Box<dyn Send + Future<Output = Result<()>>>> + Sync;

impl Instance {
	pub async fn check_out(
		&self,
		artifact_hash: artifact::Hash,
		path: &os::Path,
		reference_handler: Option<&'_ ReferenceHandlerFn>,
	) -> Result<()> {
		// Check in an existing artifact at the path.
		let existing_artifact_hash = if os::fs::exists(path).await? {
			Some(self.check_in(path).await?)
		} else {
			None
		};

		// Check out the artifact recursively.
		self.check_out_inner(
			existing_artifact_hash,
			artifact_hash,
			path,
			reference_handler,
		)
		.await?;

		Ok(())
	}

	async fn check_out_inner(
		&self,
		existing_artifact_hash: Option<artifact::Hash>,
		artifact_hash: artifact::Hash,
		path: &os::Path,
		reference_handler: Option<&'_ ReferenceHandlerFn>,
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
					existing_artifact_hash,
					directory,
					path,
					reference_handler,
				)
				.await
				.with_context(|| {
					let path = path.display();
					format!(r#"Failed to check out directory "{artifact_hash}" to "{path}"."#)
				})?;
			},
			Artifact::File(file) => {
				self.check_out_file(existing_artifact_hash, file, path)
					.await
					.with_context(|| {
						let path = path.display();
						format!(r#"Failed to check out file "{artifact_hash}" to "{path}"."#)
					})?;
			},
			Artifact::Symlink(symlink) => {
				self.check_out_symlink(existing_artifact_hash, symlink, path)
					.await
					.with_context(|| {
						let path = path.display();
						format!(r#"Failed to check out symlink "{artifact_hash}" to "{path}"."#)
					})?;
			},
			Artifact::Reference(reference) => {
				self.check_out_reference(
					existing_artifact_hash,
					reference,
					path,
					reference_handler,
				)
				.await
				.with_context(|| {
					let path = path.display();
					format!(r#"Failed to check out reference "{artifact_hash}" to "{path}"."#)
				})?;
			},
		}

		// Clear the file system object's timestamps after performing the checkout.
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

	#[async_recursion]
	async fn check_out_directory(
		&self,
		existing_artifact_hash: Option<artifact::Hash>,
		directory: Directory,
		path: &os::Path,
		reference_handler: Option<&'async_recursion ReferenceHandlerFn>,
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
				tokio::fs::create_dir(path).await?;
			},

			// If there is no artifact at this path, then create a directory.
			None => {
				tokio::fs::create_dir(path).await?;
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
						self.check_out_inner(
							existing_artifact_hash,
							entry_hash,
							&entry_path,
							reference_handler,
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
		existing_artifact_hash: Option<artifact::Hash>,
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
		existing_artifact_hash: Option<artifact::Hash>,
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
		existing_artifact_hash: Option<artifact::Hash>,
		reference: Reference,
		path: &os::Path,
		reference_handler: Option<&'async_recursion ReferenceHandlerFn>,
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

		if let Some(reference_handler) = reference_handler {
			// If there is a reference handler, then call it.
			reference_handler(&reference, path).await?;
		} else {
			// Otherwise, check out the reference to the path.
			self.check_out(reference.artifact_hash, path, None).await?;
		}

		Ok(())
	}
}

impl Instance {
	#[async_recursion]
	#[must_use]
	pub async fn check_out_internal(
		self: &Arc<Self>,
		artifact_hash: artifact::Hash,
	) -> Result<os::PathBuf> {
		// Get the checkout path.
		let checkout_path = self.checkouts_path().join(artifact_hash.to_string());

		// Perform the checkout if necessary.
		if !os::fs::exists(&checkout_path).await? {
			// Create a temp path to check out the artifact to.
			let temp_path = self.temp_path();

			// Create the callback to create reference artifact checkouts.
			let reference_handler = {
				let tg = Arc::clone(self);
				move |reference: &Reference, path: &os::Path| {
					let tg = Arc::clone(&tg);
					let reference = reference.clone();
					let path = path.to_owned();
					async move {
						// Get the target by checking out the reference.
						let mut target = tg
							.check_out_internal(reference.artifact_hash)
							.await
							.context("Failed to check out the reference.")?;

						// Add the reference path to the target.
						if let Some(reference_path) = reference.path {
							target.push(reference_path.to_string());
						}

						// Make the target relative to the symlink path.
						let parent_path = path
							.parent()
							.context("Expected the path to have a parent.")?;
						let target = pathdiff::diff_paths(target, parent_path).context(
							"Could not resolve the symlink target relative to the path.",
						)?;

						// Create the symlink.
						tokio::fs::symlink(target, path)
							.await
							.context("Failed to write the symlink for the reference.")?;

						Ok::<_, anyhow::Error>(())
					}
					.boxed()
				}
			};

			// Perform the checkout.
			self.check_out(artifact_hash, &temp_path, Some(&reference_handler))
				.await
				.context("Failed to perform the checkout.")?;

			// Move the checkout to the checkouts path.
			match tokio::fs::rename(&temp_path, &checkout_path).await {
				Ok(()) => Ok(()),

				// If the error is ENOTEMPTY or EEXIST, then we can ignore it because there is already an artifact checkout present.
				Err(error)
					if matches!(error.raw_os_error(), Some(libc::ENOTEMPTY | libc::EEXIST)) =>
				{
					Ok(())
				},

				Err(error) => Err(error),
			}
			.context("Failed to move the checkout to the checkout path.")?;
		}

		Ok(checkout_path)
	}
}
