use crate::{
	artifact::{Artifact, ArtifactHash, Dependency, Directory, File, Symlink},
	util::{path_exists, rmrf},
	watcher::Watcher,
	Cli,
};
use anyhow::{anyhow, Context, Result};
use async_recursion::async_recursion;
use futures::{future::try_join_all, Future, FutureExt};
use std::{
	os::unix::prelude::PermissionsExt,
	path::{Path, PathBuf},
	pin::Pin,
	sync::Arc,
};

pub type DependencyHandlerFn =
	dyn Sync + Fn(&Dependency, &Path) -> Pin<Box<dyn Send + Future<Output = Result<()>>>>;

impl Cli {
	pub async fn checkout(
		&self,
		artifact_hash: ArtifactHash,
		path: &Path,
		dependency_handler: Option<&'_ DependencyHandlerFn>,
	) -> Result<()> {
		// Create a watcher.
		let watcher = Watcher::new(self.path(), Arc::clone(&self.inner.file_system_semaphore));

		// Call the recursive checkout function.
		self.checkout_path(&watcher, artifact_hash, path, dependency_handler)
			.await?;

		Ok(())
	}

	async fn checkout_path(
		&self,
		watcher: &Watcher,
		artifact_hash: ArtifactHash,
		path: &Path,
		dependency_handler: Option<&'_ DependencyHandlerFn>,
	) -> Result<()> {
		// Get the artifact.
		let artifact = self.get_artifact_local(artifact_hash)?;

		// Call the appropriate function for the artifact's type.
		match artifact {
			Artifact::Directory(directory) => {
				self.checkout_directory(watcher, directory, path, dependency_handler)
					.await
					.with_context(|| {
						format!(
							"Failed to check out directory \"{artifact_hash}\" to \"{}\"",
							path.display()
						)
					})?;
			},
			Artifact::File(file) => {
				self.checkout_file(watcher, file, path)
					.await
					.with_context(|| {
						format!(
							"Failed to check out file \"{artifact_hash}\" to \"{}\"",
							path.display()
						)
					})?;
			},
			Artifact::Symlink(symlink) => {
				self.checkout_symlink(watcher, symlink, path)
					.await
					.with_context(|| {
						format!(
							"Failed to check out symlink \"{artifact_hash}\" to \"{}\"",
							path.display()
						)
					})?;
			},
			Artifact::Dependency(dependency) => {
				self.checkout_dependency(watcher, dependency, path, dependency_handler)
					.await
					.with_context(|| {
						format!(
							"Failed to check out dependency \"{artifact_hash}\" to \"{}\"",
							path.display()
						)
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
	async fn checkout_directory(
		&self,
		watcher: &Watcher,
		directory: Directory,
		path: &Path,
		dependency_handler: Option<&'async_recursion DependencyHandlerFn>,
	) -> Result<()> {
		// Handle an existing file system object at the path.
		match watcher.get(path).await? {
			// If the artifact is already checked out then return.
			Some((_, Artifact::Directory(local_directory))) if local_directory == directory => {
				return Ok(());
			},

			// If there is already a directory then remove any extraneous entries.
			Some((_, Artifact::Directory(local_directory))) => {
				try_join_all(local_directory.entries.keys().map(|entry_name| {
					let directory = &directory;
					async move {
						if !directory.entries.contains_key(entry_name) {
							let entry_path = path.join(entry_name);
							rmrf(&entry_path, None).await?;
						}
						Ok::<_, anyhow::Error>(())
					}
				}))
				.await?;
			},

			// If there is an existing file system object at the path and it is not a directory, then remove it, create a directory, and continue.
			Some(_) => {
				rmrf(path, None).await?;
				tokio::fs::create_dir(path).await?;
			},

			// If there is no file system object at this path then create a directory.
			None => {
				tokio::fs::create_dir(path).await?;
			},
		};

		// Recurse into the children.
		try_join_all(
			directory
				.entries
				.into_iter()
				.map(|(entry_name, entry_hash)| async move {
					let entry_path = path.join(&entry_name);
					self.checkout_path(watcher, entry_hash, &entry_path, dependency_handler)
						.await?;
					Ok::<_, anyhow::Error>(())
				}),
		)
		.await?;

		Ok(())
	}

	async fn checkout_file(&self, watcher: &Watcher, file: File, path: &Path) -> Result<()> {
		// Handle an existing file system object at the path.
		match watcher.get(path).await? {
			// If the artifact is already checked out then return.
			Some((_, Artifact::File(local_file))) if local_file == file => {
				return Ok(());
			},

			// If there is an existing file system object at the path then remove it and continue.
			Some(_) => {
				rmrf(path, None).await?;
			},

			// If there is no file system object at this path then continue.
			None => {},
		};

		// Copy the blob to the path.
		let output =
			std::fs::File::create(path).context("Failed to create the file to checkout to.")?;
		self.copy_blob(file.blob, output)
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

	async fn checkout_symlink(
		&self,
		watcher: &Watcher,
		symlink: Symlink,
		path: &Path,
	) -> Result<()> {
		// Handle an existing file system object at the path.
		match watcher.get(path).await? {
			// If the artifact is already checked out then return.
			Some((_, Artifact::Symlink(local_symlink))) if local_symlink == symlink => {
				return Ok(());
			},

			// If there is an existing file system object at the path then remove it and continue.
			Some(_) => {
				rmrf(path, None).await?;
			},

			// If there is no file system object at this path then continue.
			None => {},
		};

		// Create the symlink.
		tokio::fs::symlink(symlink.target, path).await?;

		Ok(())
	}

	#[async_recursion]
	async fn checkout_dependency(
		&self,
		watcher: &Watcher,
		dependency: Dependency,
		path: &Path,
		dependency_handler: Option<&'async_recursion DependencyHandlerFn>,
	) -> Result<()> {
		// Handle an existing file system object at the path.
		match watcher.get(path).await? {
			// If the artifact is already checked out then return.
			Some((_, Artifact::Dependency(local_dependency))) if local_dependency == dependency => {
				return Ok(());
			},

			// If there is an existing file system object at the path then remove it and continue.
			Some(_) => {
				rmrf(path, None).await?;
			},

			// If there is no file system object at this path then continue.
			None => {},
		};

		if let Some(dependency_handler) = dependency_handler {
			// If there is a dependency handler, call it.
			dependency_handler(&dependency, path).await?;
		} else {
			// Otherwise, check out the dependency to the path.
			self.checkout(dependency.artifact, path, None).await?;
		}

		Ok(())
	}
}

impl Cli {
	#[async_recursion]
	#[must_use]
	pub async fn checkout_internal(&self, artifact_hash: ArtifactHash) -> Result<PathBuf> {
		// Get the checkout path.
		let checkout_path = self.checkouts_path().join(artifact_hash.to_string());

		// Perform the checkout if necessary.
		if !path_exists(&checkout_path).await? {
			// Create a temp path to check out the artifact to.
			let temp_path = self.temp_path();

			// Create the callback to create dependency artifact checkouts.
			let dependency_handler = {
				let cli = self.clone();
				move |dependency: &Dependency, path: &Path| {
					let cli = cli.clone();
					let dependency = dependency.clone();
					let path = path.to_owned();
					async move {
						// Get the target by checking out the dependency.
						let mut target = cli
							.checkout_internal(dependency.artifact)
							.await
							.context("Failed to check out the dependency.")?;

						// Add the dependency path to the target.
						if let Some(dependency_path) = dependency.path {
							target.push(dependency_path);
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
							.context("Failed to write the symlink for the dependency.")?;

						Ok::<_, anyhow::Error>(())
					}
					.boxed()
				}
			};

			// Perform the checkout.
			self.checkout(artifact_hash, &temp_path, Some(&dependency_handler))
				.await
				.context("Failed to perform the checkout.")?;

			// Move the checkout to the checkouts path.
			match tokio::fs::rename(&temp_path, &checkout_path).await {
				Ok(()) => {},

				// If the error is ENOTEMPTY or EEXIST then we can ignore it because there is already an artifact checkout present.
				Err(error)
					if matches!(error.raw_os_error(), Some(libc::ENOTEMPTY | libc::EEXIST)) => {},

				Err(error) => {
					return Err(
						anyhow!(error).context("Failed to move the checkout to the checkout path.")
					);
				},
			};
		}

		Ok(checkout_path)
	}
}
