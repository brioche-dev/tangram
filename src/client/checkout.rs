use super::{cache::Cache, Transport};
use crate::{
	client::Client,
	expression::{Artifact, Dependency, Directory, Expression, File, Symlink},
	hash::Hash,
	util::rmrf,
};
use anyhow::{anyhow, Result};
use async_recursion::async_recursion;
use futures::{future::try_join_all, Future, TryStreamExt};
use std::{os::unix::prelude::PermissionsExt, path::Path, pin::Pin, sync::Arc};
use tokio_util::io::StreamReader;

pub type DependencyHandlerFn =
	dyn Sync + Fn(&Dependency, &Path) -> Pin<Box<dyn Send + Future<Output = Result<()>>>>;

impl Client {
	pub async fn checkout(
		&self,
		artifact: Artifact,
		path: &Path,
		dependency_handler: Option<&'_ DependencyHandlerFn>,
	) -> Result<()> {
		// Create a cache.
		let cache = Cache::new(path, Arc::clone(&self.file_system_semaphore));

		// Call the recursive checkout function on the root expression.
		self.checkout_path(&cache, artifact.hash, path, dependency_handler)
			.await?;

		Ok(())
	}

	async fn checkout_path(
		&self,
		cache: &Cache,
		hash: Hash,
		path: &Path,
		dependency_handler: Option<&'_ DependencyHandlerFn>,
	) -> Result<()> {
		// Get the expression from the server.
		let expression = match self.transport.as_in_process_or_http() {
			super::transport::InProcessOrHttp::InProcess(server) => {
				server.try_get_expression(hash).await?
			},
			super::transport::InProcessOrHttp::Http(http) => {
				let path = format!("/expressions/{hash}");
				http.get_json(&path).await?
			},
		};
		let expression =
			expression.ok_or_else(|| anyhow!(r#"Failed to find expression "{hash}"."#))?;

		// Call the appropriate function for the expression's type.
		match expression {
			Expression::Directory(directory) => {
				self.checkout_directory(cache, directory, path, dependency_handler)
					.await?;
			},
			Expression::File(file) => {
				self.checkout_file(cache, file, path).await?;
			},
			Expression::Symlink(symlink) => {
				self.checkout_symlink(cache, symlink, path).await?;
			},
			Expression::Dependency(dependency) => {
				self.checkout_dependency(cache, dependency, path, dependency_handler)
					.await?;
			},
			_ => unreachable!(),
		}

		Ok(())
	}

	#[async_recursion]
	async fn checkout_directory(
		&self,
		cache: &Cache,
		directory: Directory,
		path: &Path,
		dependency_handler: Option<&'async_recursion DependencyHandlerFn>,
	) -> Result<()> {
		// Handle an existing file system object at the path.
		match cache.get(path).await? {
			// If the expression is already checked out then return.
			Some((_, Expression::Directory(local_directory))) if local_directory == directory => {
				return Ok(());
			},

			// If there is already a directory then remove any entries in the local directory that are not present in the remote directory.
			Some((_, Expression::Directory(local_directory))) => {
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
					self.checkout_path(cache, entry_hash, &entry_path, dependency_handler)
						.await?;
					Ok::<_, anyhow::Error>(())
				}),
		)
		.await?;

		Ok(())
	}

	async fn checkout_file(&self, cache: &Cache, file: File, path: &Path) -> Result<()> {
		// Handle an existing file system object at the path.
		match cache.get(path).await? {
			// If the expression is already checked out then return.
			Some((_, Expression::File(local_file))) if local_file == file => {
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

	async fn checkout_symlink(&self, cache: &Cache, symlink: Symlink, path: &Path) -> Result<()> {
		// Handle an existing file system object at the path.
		match cache.get(path).await? {
			// If the expression is already checked out then return.
			Some((_, Expression::Symlink(local_symlink))) if local_symlink == symlink => {
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
		cache: &Cache,
		dependency: Dependency,
		path: &Path,
		dependency_handler: Option<&'async_recursion DependencyHandlerFn>,
	) -> Result<()> {
		// Handle an existing file system object at the path.
		match cache.get(path).await? {
			// If the expression is already checked out then return.
			Some((_, Expression::Dependency(local_dependency)))
				if local_dependency == dependency =>
			{
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
			// Otherwise, checkout the dependency to the path.
			self.checkout(dependency.artifact, path, None).await?;
		}

		Ok(())
	}
}
