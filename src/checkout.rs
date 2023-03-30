use crate::{
	artifact::{self, Artifact},
	directory::Directory,
	error::{return_error, Error, Result, WrapErr},
	file::File,
	symlink::Symlink,
	temp::Temp,
	template,
	util::{fs, task_map::TaskMap},
	Instance,
};
use async_recursion::async_recursion;
use futures::{future::try_join_all, FutureExt};
use std::{
	os::unix::prelude::PermissionsExt,
	sync::{Arc, Weak},
};

impl Instance {
	pub async fn check_out_internal(
		self: &Arc<Self>,
		artifact_hash: artifact::Hash,
	) -> Result<fs::PathBuf> {
		// Get the internal checkouts task map.
		let internal_checkouts_task_map = self
			.internal_checkouts_task_map
			.lock()
			.unwrap()
			.get_or_insert_with(|| {
				Arc::new(TaskMap::new(Box::new({
					let tg = Arc::downgrade(self);
					move |artifact_hash| {
						let tg = Weak::clone(&tg);
						async move {
							let tg = Weak::upgrade(&tg).unwrap();
							tg.check_out_internal_inner(artifact_hash).await
						}
						.boxed()
					}
				})))
			})
			.clone();

		// Perform the checkout.
		let path = internal_checkouts_task_map.run(artifact_hash).await?;

		Ok(path)
	}

	pub async fn check_out_internal_inner(
		&self,
		artifact_hash: artifact::Hash,
	) -> Result<fs::PathBuf> {
		// Compute the checkout's path in the artifacts directory.
		let path = self.artifacts_path().join(artifact_hash.to_string());

		// If the path exists, then the artifact is already checked out.
		if crate::util::fs::exists(&path).await? {
			return Ok(path);
		}

		// Create a temp.
		let temp = Temp::new(self);

		// Perform the checkout to the temp path.
		self.check_out_internal_inner_inner(artifact_hash, temp.path())
			.await?;

		// Move the checkout from the temp path to the path in the artifacts directory.
		match tokio::fs::rename(temp.path(), &path).await {
			Ok(()) => Ok(()),

			// If the error is ENOTEMPTY or EEXIST, then we can ignore it because there is already an artifact checkout present.
			Err(error) if matches!(error.raw_os_error(), Some(libc::ENOTEMPTY | libc::EEXIST)) => {
				Ok(())
			},

			Err(error) => Err(error),
		}
		.wrap_err("Failed to move the checkout to the checkout path.")?;

		// Clear the file system object's timestamps.
		tokio::task::spawn_blocking({
			let path = path.clone();
			move || {
				let epoch = filetime::FileTime::from_unix_time(0, 0);
				filetime::set_symlink_file_times(path, epoch, epoch)
					.wrap_err("Failed to set the file system object's timestamps.")?;
				Ok::<_, Error>(())
			}
		})
		.await
		.unwrap()?;

		Ok(path)
	}

	#[async_recursion]
	async fn check_out_internal_inner_inner(
		&self,
		artifact_hash: artifact::Hash,
		path: &fs::Path,
	) -> Result<()> {
		// Get the artifact.
		let artifact = self.get_artifact_local(artifact_hash)?;

		match &artifact {
			Artifact::Directory(directory) => {
				// Create the directory.
				tokio::fs::create_dir(path).await?;

				// Recurse into the entries.
				try_join_all(
					directory
						.entries
						.iter()
						.map(|(entry_name, entry_hash)| async move {
							let entry_path = path.join(entry_name);
							self.check_out_internal_inner_inner(*entry_hash, &entry_path)
								.await?;
							Ok::<_, Error>(())
						}),
				)
				.await?;
			},

			Artifact::File(file) => {
				// Copy the blob to the path.
				self.copy_blob_to_path(file.blob_hash, path)
					.await
					.wrap_err("Failed to copy the blob.")?;

				// Make the file executable if necessary.
				if file.executable {
					let permissions = std::fs::Permissions::from_mode(0o755);
					tokio::fs::set_permissions(path, permissions).await?;
				}

				// Check out the references.
				try_join_all(file.references.iter().map(|artifact_hash| async move {
					self.check_out_internal_inner(*artifact_hash).await?;
					Ok::<_, Error>(())
				}))
				.await?;
			},

			Artifact::Symlink(symlink) => {
				// Render the symlink target.
				let target = symlink
					.target
					.render(|component| async move {
						match component {
							template::Component::String(string) => Ok(string.into()),

							template::Component::Artifact(artifact_hash) => {
								// Check out the artifact.
								let artifact_path =
									self.check_out_internal_inner(*artifact_hash).await?;

								// Resolve the symlink target relative to the path.
								let artifact_target_path = pathdiff::diff_paths(
									artifact_path,
									path.parent().unwrap(),
								)
								.wrap_err(
									"Could not resolve the symlink target relative to the path.",
								)?;

								// Convert the path to a string.
								let string = artifact_target_path
									.into_os_string()
									.into_string()
									.unwrap()
									.into();

								Ok(string)
							},

							template::Component::Placeholder(_) => Err(Error::message(
								"Symlink target template contains a placeholder.",
							)),
						}
					})
					.await?;

				// Create the symlink.
				tokio::fs::symlink(target, path)
					.await
					.wrap_err("Failed to write the symlink for the reference.")?;
			},
		};

		// Clear the file system object's timestamps.
		tokio::task::spawn_blocking({
			let path = path.to_owned();
			move || {
				let epoch = filetime::FileTime::from_unix_time(0, 0);
				filetime::set_symlink_file_times(path, epoch, epoch)
					.wrap_err("Failed to set the file system object's timestamps.")?;
				Ok::<_, Error>(())
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
		path: &fs::Path,
	) -> Result<()> {
		// Check in an existing artifact at the path.
		let existing_artifact_hash = if crate::util::fs::exists(path).await? {
			Some(self.check_in(path).await?)
		} else {
			None
		};

		// Check out the artifact recursively.
		self.check_out_external_inner(existing_artifact_hash, artifact_hash, path)
			.await?;

		Ok(())
	}

	async fn check_out_external_inner(
		&self,
		existing_artifact_hash: Option<artifact::Hash>,
		artifact_hash: artifact::Hash,
		path: &fs::Path,
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
				self.check_out_directory(existing_artifact_hash, artifact_hash, directory, path)
					.await
					.wrap_err_with(|| {
						let path = path.display();
						format!(r#"Failed to check out directory "{artifact_hash}" to "{path}"."#)
					})?;
			},

			Artifact::File(file) => {
				self.check_out_file(existing_artifact_hash, artifact_hash, file, path)
					.await
					.wrap_err_with(|| {
						let path = path.display();
						format!(r#"Failed to check out file "{artifact_hash}" to "{path}"."#)
					})?;
			},

			Artifact::Symlink(symlink) => {
				self.check_out_symlink(existing_artifact_hash, artifact_hash, symlink, path)
					.await
					.wrap_err_with(|| {
						let path = path.display();
						format!(r#"Failed to check out symlink "{artifact_hash}" to "{path}"."#)
					})?;
			},
		}

		Ok(())
	}

	#[async_recursion]
	async fn check_out_directory(
		&self,
		existing_artifact_hash: Option<artifact::Hash>,
		_artifact_hash: artifact::Hash,
		directory: Directory,
		path: &fs::Path,
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
							crate::util::fs::rmrf(&entry_path).await?;
						}
						Ok::<_, Error>(())
					}
				}))
				.await?;
			},

			// If there is an existing artifact at the path and it is not a directory, then remove it, create a directory, and continue.
			Some(_) => {
				crate::util::fs::rmrf(path).await?;
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
							existing_artifact_hash,
							entry_hash,
							&entry_path,
						)
						.await?;

						Ok::<_, Error>(())
					}
				}),
		)
		.await?;

		Ok(())
	}

	async fn check_out_file(
		&self,
		existing_artifact_hash: Option<artifact::Hash>,
		_artifact_hash: artifact::Hash,
		file: File,
		path: &fs::Path,
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
				crate::util::fs::rmrf(path).await?;
			},

			// If there is no file system object at this path, then continue.
			None => {},
		};

		// Copy the blob to the path.
		self.copy_blob_to_path(file.blob_hash, path)
			.await
			.wrap_err("Failed to copy the blob.")?;

		// Make the file executable if necessary.
		if file.executable {
			let permissions = std::fs::Permissions::from_mode(0o755);
			tokio::fs::set_permissions(path, permissions).await?;
		}

		// Check that the file has no references.
		if !file.references.is_empty() {
			return_error!(r#"Cannot check out a file with references."#);
		}

		Ok(())
	}

	async fn check_out_symlink(
		&self,
		existing_artifact_hash: Option<artifact::Hash>,
		_artifact_hash: artifact::Hash,
		symlink: Symlink,
		path: &fs::Path,
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
				crate::util::fs::rmrf(path).await?;
			},

			// If there is no file system object at this path, then continue.
			None => {},
		};

		// Render the target.
		let target = symlink
			.target
			.render(|component| async move {
				match component {
					crate::template::Component::String(string) => Ok(string.into()),
					_ => Err(Error::message(
						"Cannot check out a symlink whose target has non-string components.",
					)),
				}
			})
			.await?;

		// Create the symlink.
		tokio::fs::symlink(target, path).await?;

		Ok(())
	}
}
