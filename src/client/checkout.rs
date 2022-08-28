use super::{object_cache::ObjectCache, Transport};
use crate::{
	artifact::Artifact,
	client::Client,
	object::{Dependency, Directory, File, Object, ObjectHash, Symlink},
	util::rmrf,
};
use anyhow::{anyhow, Result};
use async_recursion::async_recursion;
use futures::Future;
use std::{
	os::unix::prelude::PermissionsExt,
	path::{Path, PathBuf},
	pin::Pin,
	sync::Arc,
};

pub type ExternalPathForDependencyFn =
	dyn Sync + Fn(&Dependency) -> Pin<Box<dyn Send + Future<Output = Result<Option<PathBuf>>>>>;

impl Client {
	pub async fn checkout(
		&self,
		artifact: &Artifact,
		path: &Path,
		external_path_for_dependency: Option<&'_ ExternalPathForDependencyFn>,
	) -> Result<()> {
		// Create an object cache.
		let object_cache = ObjectCache::new(path, Arc::clone(&self.file_system_semaphore));

		// Call the recursive checkout function on the root object.
		self.checkout_path(
			&object_cache,
			artifact.object_hash,
			path,
			external_path_for_dependency,
		)
		.await?;

		Ok(())
	}

	async fn checkout_path(
		&self,
		object_cache: &ObjectCache,
		remote_object_hash: ObjectHash,
		path: &Path,
		external_path_for_dependency: Option<&'_ ExternalPathForDependencyFn>,
	) -> Result<()> {
		// Get the object from the server.
		let object = match &self.transport {
			Transport::InProcess { server } => server.get_object(remote_object_hash).await?,
			_ => unimplemented!(),
		};
		let object =
			object.ok_or_else(|| anyhow!("Failed to find object {remote_object_hash}."))?;

		// Call the appropriate function for the object's type.
		match object {
			Object::Directory(directory) => {
				self.checkout_directory(
					object_cache,
					directory,
					path,
					external_path_for_dependency,
				)
				.await?;
			},
			Object::File(file) => {
				self.checkout_file(object_cache, file, path).await?;
			},
			Object::Symlink(symlink) => {
				self.checkout_symlink(object_cache, symlink, path).await?;
			},
			Object::Dependency(dependency) => {
				self.checkout_dependency(
					object_cache,
					dependency,
					path,
					external_path_for_dependency,
				)
				.await?;
			},
		}

		Ok(())
	}

	#[async_recursion]
	async fn checkout_directory(
		&self,
		object_cache: &ObjectCache,
		directory: Directory,
		path: &Path,
		external_path_for_dependency: Option<&'async_recursion ExternalPathForDependencyFn>,
	) -> Result<()> {
		// Handle an existing file system object at the path.
		match object_cache.get(path).await? {
			// If the object is already checked out then return.
			Some((_, Object::Directory(local_directory))) if local_directory == directory => {
				return Ok(());
			},

			// If there is already a directory then remove any entries in the local directory that are not present in the remote directory.
			Some((_, Object::Directory(local_directory))) => {
				futures::future::try_join_all(local_directory.entries.keys().map(|entry_name| {
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
		futures::future::try_join_all(directory.entries.into_iter().map(
			|(entry_name, entry_object_hash)| {
				async move {
					let entry_path = path.join(&entry_name);
					self.checkout_path(
						object_cache,
						entry_object_hash,
						&entry_path,
						external_path_for_dependency,
					)
					.await?;
					Ok::<_, anyhow::Error>(())
				}
			},
		))
		.await?;

		Ok(())
	}

	async fn checkout_file(
		&self,
		object_cache: &ObjectCache,
		file: File,
		path: &Path,
	) -> Result<()> {
		// Handle an existing file system object at the path.
		match object_cache.get(path).await? {
			// If the object is already checked out then return.
			Some((_, Object::File(local_file))) if local_file == file => {
				return Ok(());
			},

			// If there is an existing file system object at the path then remove it and continue.
			Some(_) => {
				rmrf(path, None).await?;
			},

			// If there is no file system object at this path then continue.
			None => {},
		};

		// Write the file to the path.
		match &self.transport {
			Transport::InProcess { server } => {
				tokio::fs::copy(&server.blob_path(file.blob_hash), &path).await?;
			},
			_ => unimplemented!(),
		};

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
		object_cache: &ObjectCache,
		symlink: Symlink,
		path: &Path,
	) -> Result<()> {
		// Handle an existing file system object at the path.
		match object_cache.get(path).await? {
			// If the object is already checked out then return.
			Some((_, Object::Symlink(local_symlink))) if local_symlink == symlink => {
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
		object_cache: &ObjectCache,
		dependency: Dependency,
		path: &Path,
		external_path_for_dependency: Option<&'async_recursion ExternalPathForDependencyFn>,
	) -> Result<()> {
		// Handle an existing file system object at the path.
		match object_cache.get(path).await? {
			// If the object is already checked out then return.
			Some((_, Object::Dependency(local_dependency))) if local_dependency == dependency => {
				return Ok(());
			},

			// If there is an existing file system object at the path then remove it and continue.
			Some(_) => {
				rmrf(path, None).await?;
			},

			// If there is no file system object at this path then continue.
			None => {},
		};

		// Get the dependency path.
		let dependency_path = if let Some(path_for_dependency) = external_path_for_dependency {
			path_for_dependency(&dependency).await?
		} else {
			None
		};

		// Checkout the dependency.
		self.checkout(
			&dependency.artifact,
			dependency_path.as_deref().unwrap_or(path),
			external_path_for_dependency,
		)
		.await?;

		// If the dependency path is external, create a symlink.
		if let Some(dependency_path) = dependency_path {
			// Compute the target path.
			let target = dependency_path;

			// Create the symlink.
			tokio::fs::symlink(target, path).await?;
		}

		Ok(())
	}
}
