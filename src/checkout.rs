use crate::{self as tg, return_error, subpath::Subpath, Client, Error, Result, WrapErr};
use async_recursion::async_recursion;
use futures::{stream::FuturesUnordered, TryStreamExt};
use std::{os::unix::prelude::PermissionsExt, path::Path};

impl tg::Artifact {
	pub async fn check_out(&self, tg: &Client, path: &Path) -> Result<()> {
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
		tg: &Client,
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
		match self.value() {
			tg::artifact::Value::Directory(directory) => {
				Self::check_out_directory(tg, existing_artifact, &directory, path)
					.await
					.wrap_err_with(|| {
						let path = path.display();
						format!(r#"Failed to check out directory "{id}" to "{path}"."#)
					})?;
			},

			tg::artifact::Value::File(file) => {
				Self::check_out_file(tg, existing_artifact, &file, path)
					.await
					.wrap_err_with(|| {
						let path = path.display();
						format!(r#"Failed to check out file "{id}" to "{path}"."#)
					})?;
			},

			tg::artifact::Value::Symlink(symlink) => {
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
		tg: &Client,
		existing_artifact: Option<&'async_recursion tg::Artifact>,
		directory: &tg::Directory,
		path: &Path,
	) -> Result<()> {
		// Handle an existing artifact at the path.
		match existing_artifact.map(tg::Artifact::value) {
			// If there is already a directory, then remove any extraneous entries.
			Some(tg::artifact::Value::Directory(existing_directory)) => {
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
					let existing_artifact = match existing_artifact.map(tg::Artifact::value) {
						Some(tg::artifact::Value::Directory(existing_directory)) => {
							let name: Subpath = name.parse().wrap_err("Invalid entry name.")?;
							existing_directory.try_get(tg, &name).await?
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
		client: &Client,
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
		let permit = client.file_descriptor_semaphore().acquire().await;
		tokio::io::copy(
			&mut file.contents(client).await?.reader(client).await?,
			&mut tokio::fs::File::create(path).await?,
		)
		.await
		.wrap_err("Failed to copy the blob.")?;
		drop(permit);

		// Make the file executable if necessary.
		if file.executable(client).await? {
			let permissions = std::fs::Permissions::from_mode(0o755);
			tokio::fs::set_permissions(path, permissions).await?;
		}

		// Check that the file has no references.
		if !file.references(client).await?.is_empty() {
			return_error!(r#"Cannot check out a file with references."#);
		}

		Ok(())
	}

	async fn check_out_symlink(
		client: &Client,
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
			.target(client)
			.await?
			.value(client)
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
