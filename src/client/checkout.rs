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
use futures::TryStreamExt;
use std::{
	os::unix::prelude::PermissionsExt,
	path::{Path, PathBuf},
	pin::Pin,
	sync::Arc,
};
use tokio_util::io::StreamReader;

pub type ExternalPathForDependencyFn =
	dyn Sync + Fn(&Dependency) -> Pin<Box<dyn Send + Future<Output = Result<Option<PathBuf>>>>>;

impl Client {
	pub async fn checkout(
		&self,
		artifact: Artifact,
		path: &Path,
		external_path_for_dependency: Option<&'_ ExternalPathForDependencyFn>,
	) -> Result<()> {
		// Create an object cache.
		let object_cache = ObjectCache::new(path, Arc::clone(&self.file_system_semaphore));

		// Call the recursive checkout function on the root object.
		self.checkout_path(
			&object_cache,
			artifact.object_hash(),
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
		let object = match self.transport.as_in_process_or_http() {
			super::transport::InProcessOrHttp::InProcess(server) => {
				server.get_object(remote_object_hash).await?
			},
			super::transport::InProcessOrHttp::Http(http) => {
				let path = format!("/objects/{remote_object_hash}");
				http.get_json(&path).await?
			},
		};
		let object =
			object.ok_or_else(|| anyhow!(r#"Failed to find object "{remote_object_hash}"."#))?;

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
			|(entry_name, entry_object_hash)| async move {
				let entry_path = path.join(&entry_name);
				self.checkout_path(
					object_cache,
					entry_object_hash,
					&entry_path,
					external_path_for_dependency,
				)
				.await?;
				Ok::<_, anyhow::Error>(())
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

		// Get the server path if it is local.
		let local_server_path = match &self.transport {
			Transport::InProcess(server) => Some(server.path()),
			Transport::Unix(unix) => Some(unix.path.as_ref()),
			Transport::Tcp(_) => None,
		};

		if let Some(local_server_path) = local_server_path {
			// If the server is local, copy the file.
			let local_server_blob_path = local_server_path
				.join("blobs")
				.join(file.blob_hash.to_string());
			tokio::fs::copy(&local_server_blob_path, &path).await?;
		} else if let Some(http) = self.transport.as_http() {
			// Otherwise, if the server is remote, retrieve the blob and write it to the path.
			let blob_hash = file.blob_hash;
			let request_path = format!("/blobs/{blob_hash}");

			let mut response = http
				.get(&request_path)
				.await?
				.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error));

			// Create an async reader from the body.
			let mut body = StreamReader::new(&mut response);

			// Create the file to write to.
			let mut file = tokio::fs::File::create(&path).await?;

			// Read the bytes from the body into the file.
			tokio::io::copy(&mut body, &mut file).await?;
		} else {
			unreachable!()
		}

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
			dependency.artifact,
			dependency_path.as_deref().unwrap_or(path),
			external_path_for_dependency,
		)
		.await?;

		// If the dependency path is external, create a symlink.
		if let Some(dependency_path) = dependency_path {
			// Compute the target path.
			// TODO Make this a relative path.
			let target = dependency_path;

			// Create the symlink.
			tokio::fs::symlink(target, path).await?;
		}

		Ok(())
	}
}
