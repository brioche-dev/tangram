use super::{object_cache::ObjectCache, Transport};
use crate::{
	artifact::Artifact,
	client::Client,
	object::{Directory, File, Object, ObjectHash, Symlink},
	util::rmrf,
};
use anyhow::{anyhow, Result};
use async_recursion::async_recursion;
use std::{os::unix::prelude::PermissionsExt, path::Path, sync::Arc};

impl Client {
	pub async fn checkout(&self, artifact: &Artifact, path: &Path) -> Result<()> {
		let mut object_cache = ObjectCache::new(Arc::clone(&self.file_system_semaphore));
		self.checkout_path(&mut object_cache, artifact.object_hash, path)
			.await?;
		Ok(())
	}

	async fn checkout_path(
		&self,
		object_cache: &mut ObjectCache,
		remote_object_hash: ObjectHash,
		path: &Path,
	) -> Result<()> {
		let object = match &self.transport {
			Transport::InProcess { server } => server.get_object(remote_object_hash).await?,
			_ => unimplemented!(),
		};
		let object =
			object.ok_or_else(|| anyhow!("Failed to find object {remote_object_hash}."))?;

		match object {
			Object::Directory(directory) => {
				self.checkout_directory(object_cache, directory, path)
					.await?;
			},
			Object::File(file) => {
				self.checkout_file(object_cache, file, path).await?;
			},
			Object::Symlink(symlink) => {
				self.checkout_symlink(object_cache, symlink, path).await?;
			},
			Object::Dependency(_) => todo!(),
		}

		Ok(())
	}

	#[async_recursion]
	async fn checkout_directory(
		&self,
		object_cache: &mut ObjectCache,
		directory: Directory,
		path: &Path,
	) -> Result<()> {
		match object_cache.get(path).await? {
			// If the object is already checked out then return.
			Some((_, Object::Directory(local_directory))) if local_directory == &directory => {
				return Ok(());
			},
			// If there is already a directory then remove any entries in the local directory that are not present in the remote directory.
			Some((_, Object::Directory(local_directory))) => {
				for entry_name in local_directory.entries.keys() {
					if !directory.entries.contains_key(entry_name) {
						let entry_path = path.join(&entry_name);
						rmrf(&entry_path, None).await?;
					}
				}
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
		for (entry_name, entry_object_hash) in directory.entries {
			let entry_path = path.join(&entry_name);
			self.checkout_path(object_cache, entry_object_hash, &entry_path)
				.await?;
		}

		Ok(())
	}

	async fn checkout_file(
		&self,
		object_cache: &mut ObjectCache,
		file: File,
		path: &Path,
	) -> Result<()> {
		match object_cache.get(path).await? {
			// If the object is already checked out then return.
			Some((_, Object::File(local_file))) if local_file == &file => {
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
		object_cache: &mut ObjectCache,
		symlink: Symlink,
		path: &Path,
	) -> Result<()> {
		match object_cache.get(path).await? {
			// If the object is already checked out then return.
			Some((_, Object::Symlink(local_symlink))) if local_symlink == &symlink => {
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
}
