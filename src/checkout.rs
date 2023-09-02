use crate::{
	self as tg,
	artifact::Artifact,
	error::{return_error, Error, Result, WrapErr},
	instance::Instance,
	subpath::Subpath,
	temp::Temp,
	template,
	util::task_map::TaskMap,
};
use async_recursion::async_recursion;
use futures::{stream::FuturesUnordered, FutureExt, TryStreamExt};
use std::{
	os::unix::prelude::PermissionsExt,
	path::{Path, PathBuf},
	sync::Arc,
};

impl tg::Artifact {
	pub async fn check_out_internal(&self, tg: &Instance) -> Result<PathBuf> {
		// Get the internal checkouts task map.
		let internal_checkouts_task_map = tg
			.internal_checkouts_task_map
			.lock()
			.unwrap()
			.get_or_insert_with(|| {
				Arc::new(TaskMap::new(Box::new({
					let state = Arc::downgrade(&tg.state);
					move |id| {
						let state = state.clone();
						async move {
							let state = state.upgrade().unwrap();
							let tg = Instance { state };
							let artifact = tg::Artifact::with_id(id)?;
							artifact.check_out_internal_inner(&tg).await
						}
						.boxed()
					}
				})))
			})
			.clone();

		// Perform the checkout.
		let id = self.id(tg).await?;
		let path = internal_checkouts_task_map.run(id).await?;

		Ok(path)
	}

	async fn check_out_internal_inner(&self, tg: &Instance) -> Result<PathBuf> {
		// Compute the checkout's path in the artifacts directory.
		let id = self.id(tg).await?;
		let path = tg.artifacts_path().join(id.to_string());

		// If the path exists, then the artifact is already checked out.
		if tokio::fs::try_exists(&path).await? {
			return Ok(path);
		}

		// Create a temp.
		let temp = Temp::new(tg);

		// Perform the checkout to the temp path.
		self.check_out_internal_inner_inner(tg, temp.path()).await?;

		// // If the artifact is a directory, then make it writeable so it can be renamed.
		// if self.as_directory().is_some() {
		// 	tokio::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o755))
		// 		.await
		// 		.wrap_err("Failed to make the directory writeable.")?;
		// }

		// Move the checkout from the temp path to the path in the artifacts directory.
		match tokio::fs::rename(temp.path(), &path).await {
			Ok(()) => Ok(()),

			// If the error is ENOTEMPTY or EEXIST, then ignore it because there is already an artifact checkout present.
			Err(error) if matches!(error.raw_os_error(), Some(libc::ENOTEMPTY | libc::EEXIST)) => {
				Ok(())
			},

			Err(error) => Err(error),
		}
		.wrap_err("Failed to move the checkout to the checkout path.")?;

		// // If the artifact is a directory, then make it readonly again.
		// if self.as_directory().is_some() {
		// 	tokio::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o555))
		// 		.await
		// 		.wrap_err("Failed to make the directory readonly.")?;
		// }

		// Clear the path's timestamps.
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
	#[allow(clippy::too_many_lines)]
	async fn check_out_internal_inner_inner(&self, tg: &Instance, path: &Path) -> Result<()> {
		match self.get() {
			Artifact::Directory(directory) => {
				// Create the directory.
				tokio::fs::create_dir(path)
					.await
					.wrap_err("Failed to create the directory.")?;

				// Recurse into the entries.
				directory
					.entries(tg)
					.await
					.wrap_err("Failed to get the directory's entries.")?
					.iter()
					.map(|(name, artifact)| async move {
						artifact
							.check_out_internal_inner_inner(tg, &path.join(name))
							.await?;
						Ok::<_, Error>(())
					})
					.collect::<FuturesUnordered<_>>()
					.try_collect()
					.await?;
			},

			Artifact::File(file) => {
				// Copy the blob to the path.
				let permit = tg.file_descriptor_semaphore.acquire().await;
				tokio::io::copy(
					&mut file.contents(tg).await?.reader(tg).await?,
					&mut tokio::fs::File::create(path).await?,
				)
				.await
				.wrap_err("Failed to copy the blob.")?;
				drop(permit);

				// Make the file executable if necessary.
				if file.executable(tg).await? {
					let permissions = std::fs::Permissions::from_mode(0o755);
					tokio::fs::set_permissions(path, permissions)
						.await
						.wrap_err("Failed to make the file executable.")?;
				}

				// Check out the references.
				file.references(tg)
					.await
					.wrap_err("Failed to get the file's references.")?
					.iter()
					.map(|artifact| async move {
						artifact.check_out_internal_inner(tg).await?;
						Ok::<_, Error>(())
					})
					.collect::<FuturesUnordered<_>>()
					.try_collect()
					.await?;
			},

			Artifact::Symlink(symlink) => {
				// Render the symlink target.
				let target = symlink
					.target(tg)
					.await?
					.get(tg)
					.await?
					.try_render(|component| async move {
						match component {
							template::Component::String(string) => Ok(string.into()),

							template::Component::Artifact(artifact) => {
								// Check out the artifact.
								let artifact_path = artifact.check_out_internal_inner(tg).await?;

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

		// // Make the file system object readonly.
		// let mode = match self {
		// 	Artifact::Directory(_) => Some(0o555),
		// 	Artifact::File(file) if file.executable() => Some(0o555),
		// 	Artifact::File(_) => Some(0o444),
		// 	Artifact::Symlink(_) => None,
		// };
		// if let Some(mode) = mode {
		// 	tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
		// 		.await
		// 		.wrap_err("Failed to make the file system object readonly.")?;
		// }

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

impl tg::Artifact {
	pub async fn check_out(&self, tg: &Instance, path: &Path) -> Result<()> {
		// Bundle the artifact.
		let artifact = self.clone();
		// let artifact = self
		// 	.bundle(tg)
		// 	.await
		// 	.wrap_err("Failed to bundle the artifact.")?;

		// Check in an existing artifact at the path.
		let existing_artifact = if tokio::fs::try_exists(path).await? {
			Some(Self::check_in(tg, path).await?)
		} else {
			None
		};

		// Check out the artifact recursively.
		artifact
			.check_out_inner(tg, existing_artifact.as_ref(), path)
			.await?;

		Ok(())
	}

	async fn check_out_inner(
		&self,
		tg: &Instance,
		existing_artifact: Option<&tg::Artifact>,
		path: &Path,
	) -> Result<()> {
		// If the artifact is the same as the existing artifact, then return.
		let id = self.id(tg).await?;
		match existing_artifact {
			None => {},
			Some(existing_artifact) => {
				if id == existing_artifact.id(tg).await? {
					return Ok(());
				}
			},
		}

		// Call the appropriate function for the artifact's type.
		match self.get() {
			Artifact::Directory(directory) => {
				Self::check_out_directory(tg, existing_artifact, &directory, path)
					.await
					.wrap_err_with(|| {
						let path = path.display();
						format!(r#"Failed to check out directory "{id}" to "{path}"."#)
					})?;
			},

			Artifact::File(file) => {
				Self::check_out_file(tg, existing_artifact, &file, path)
					.await
					.wrap_err_with(|| {
						let path = path.display();
						format!(r#"Failed to check out file "{id}" to "{path}"."#)
					})?;
			},

			Artifact::Symlink(symlink) => {
				Self::check_out_symlink(tg, existing_artifact, &symlink, path)
					.await
					.wrap_err_with(|| {
						let path = path.display();
						format!(r#"Failed to check out symlink "{id}" to "{path}"."#)
					})?;
			},
		}

		Ok(())
	}

	#[async_recursion]
	async fn check_out_directory(
		tg: &Instance,
		existing_artifact: Option<&'async_recursion tg::Artifact>,
		directory: &tg::Directory,
		path: &Path,
	) -> Result<()> {
		// Handle an existing artifact at the path.
		match existing_artifact.map(tg::Artifact::get) {
			// If there is already a directory, then remove any extraneous entries.
			Some(Artifact::Directory(existing_directory)) => {
				existing_directory
					.entries(tg)
					.await?
					.iter()
					.map(|(name, _)| async move {
						if !directory.entries(tg).await?.contains_key(name) {
							let entry_path = path.join(name);
							crate::util::fs::rmrf(&entry_path).await?;
						}
						Ok::<_, Error>(())
					})
					.collect::<FuturesUnordered<_>>()
					.try_collect()
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
		}

		// Recurse into the entries.
		directory
			.entries(tg)
			.await?
			.iter()
			.map(|(name, artifact)| {
				let existing_artifact = &existing_artifact;
				async move {
					// Retrieve an existing artifact.
					let existing_artifact = match existing_artifact.map(tg::Artifact::get) {
						Some(Artifact::Directory(existing_directory)) => {
							let name: Subpath = name.parse().wrap_err("Invalid entry name.")?;
							existing_directory.try_get_entry(tg, &name).await?
						},
						_ => None,
					};

					// Recurse.
					let entry_path = path.join(name);
					artifact
						.check_out_inner(tg, existing_artifact.as_ref(), &entry_path)
						.await?;

					Ok::<_, Error>(())
				}
			})
			.collect::<FuturesUnordered<_>>()
			.try_collect()
			.await?;

		Ok(())
	}

	async fn check_out_file(
		tg: &Instance,
		existing_artifact: Option<&tg::Artifact>,
		file: &tg::File,
		path: &Path,
	) -> Result<()> {
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
		let permit = tg.file_descriptor_semaphore.acquire().await;
		tokio::io::copy(
			&mut file.contents(tg).await?.reader(tg).await?,
			&mut tokio::fs::File::create(path).await?,
		)
		.await
		.wrap_err("Failed to copy the blob.")?;
		drop(permit);

		// Make the file executable if necessary.
		if file.executable(tg).await? {
			let permissions = std::fs::Permissions::from_mode(0o755);
			tokio::fs::set_permissions(path, permissions).await?;
		}

		// Check that the file has no references.
		if !file.references(tg).await?.is_empty() {
			return_error!(r#"Cannot check out a file with references."#);
		}

		Ok(())
	}

	async fn check_out_symlink(
		tg: &Instance,
		existing_artifact: Option<&tg::Artifact>,
		symlink: &tg::Symlink,
		path: &Path,
	) -> Result<()> {
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
			.target(tg)
			.await?
			.get(tg)
			.await?
			.try_render(|component| async move {
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
