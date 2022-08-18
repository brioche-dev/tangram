use crate::{
	hash::Hasher,
	object::{BlobHash, Dependency, Directory, File, Object, ObjectHash, Symlink},
};
use anyhow::{anyhow, bail, Context, Result};
use async_recursion::async_recursion;
use camino::Utf8PathBuf;
use indexmap::IndexMap;
use std::{
	collections::BTreeMap, fs::Metadata, os::unix::prelude::PermissionsExt, path::Path,
	path::PathBuf, sync::Arc,
};

pub struct ObjectCache {
	pub semaphore: Arc<tokio::sync::Semaphore>,
	pub cache: IndexMap<PathBuf, (ObjectHash, Object)>,
}

impl ObjectCache {
	pub fn new(semaphore: Arc<tokio::sync::Semaphore>) -> ObjectCache {
		let cache = IndexMap::new();
		ObjectCache { semaphore, cache }
	}

	pub async fn get(&mut self, path: &Path) -> Result<Option<&(ObjectHash, Object)>> {
		// Fill the cache for this path if necessary.
		if self.cache.get(path).is_none() {
			tracing::info!(r#"Filling the object cache for path "{path:?}""."#);

			// Get the metadata for the path.
			let permit = self.semaphore.acquire().await.unwrap();
			let metadata = match tokio::fs::symlink_metadata(path).await {
				Ok(metadata) => metadata,
				Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
				Err(error) => return Err(error.into()),
			};
			drop(permit);

			// Call the appropriate function for the file system object the path points to.
			if metadata.is_dir() {
				self.cache_object_for_directory(path, &metadata).await?;
			} else if metadata.is_file() {
				self.cache_object_for_file(path, &metadata).await?;
			} else if metadata.is_symlink() {
				// Read the "user.tangram_dependency" xattr.
				let permit = self.semaphore.acquire().await?;
				let dependency =
					if let Some(dependency) = xattr::get(path, "user.tangram_dependency")? {
						let dependency = serde_json::from_slice(&dependency)?;
						Some(dependency)
					} else {
						None
					};
				drop(permit);

				match dependency {
					None => {
						self.cache_object_for_symlink(path, &metadata).await?;
					},
					Some(dependency) => {
						self.cache_object_for_dependency(path, &metadata, dependency)
							.await?;
					},
				}
			} else {
				bail!("The path must point to a directory, file, or symlink.");
			};
		}

		// Retrieve and return the entry.
		let entry = self.cache.get(path).unwrap();
		Ok(Some(entry))
	}

	#[async_recursion]
	async fn cache_object_for_directory(
		&mut self,
		path: &Path,
		_metadata: &Metadata,
	) -> Result<()> {
		// Read the contents of the directory.
		let permit = self.semaphore.acquire().await.unwrap();
		let mut read_dir = tokio::fs::read_dir(path)
			.await
			.context("Failed to read the directory.")?;
		let mut entry_names = Vec::new();
		while let Some(entry) = read_dir.next_entry().await? {
			let file_name = entry
				.file_name()
				.to_str()
				.ok_or_else(|| anyhow!("All file names must be valid UTF-8."))?
				.to_owned();
			entry_names.push(file_name);
		}
		drop(permit);

		// Recurse into the directory's entries.
		let mut entries = BTreeMap::new();
		for entry_name in entry_names {
			let entry_path = path.join(&entry_name);
			self.get(&entry_path).await?;
			let (object_hash, _) = self.cache.get(&entry_path).unwrap();
			entries.insert(entry_name, *object_hash);
		}

		// Create the object.
		let object = Object::Directory(Directory { entries });
		let object_hash = object.hash();
		self.cache.insert(path.to_owned(), (object_hash, object));

		Ok(())
	}

	async fn cache_object_for_file(
		&mut self,
		path: &Path,
		metadata: &Metadata,
	) -> Result<ObjectHash> {
		// Compute the file's blob hash.
		let permit = self.semaphore.acquire().await.unwrap();
		let mut file = tokio::fs::File::open(path).await?;
		let mut hasher = Hasher::new();
		tokio::io::copy(&mut file, &mut hasher).await?;
		let blob_hash = BlobHash(hasher.finalize());
		drop(permit);

		// Determine if the file is executable.
		let executable = (metadata.permissions().mode() & 0o111) != 0;

		let object = Object::File(File {
			blob_hash,
			executable,
		});
		let object_hash = object.hash();
		self.cache.insert(path.to_owned(), (object_hash, object));

		Ok(object_hash)
	}

	async fn cache_object_for_symlink(
		&mut self,
		path: &Path,
		_metadata: &Metadata,
	) -> Result<ObjectHash> {
		// Read the symlink.
		let permit = self.semaphore.acquire().await.unwrap();
		let target = tokio::fs::read_link(path).await?;
		let target = Utf8PathBuf::from_path_buf(target)
			.map_err(|_| anyhow!("Symlink content is not valid UTF-8."))?;
		drop(permit);

		// Create the object and add it to the cache.
		let object = Object::Symlink(Symlink { target });
		let object_hash = object.hash();
		self.cache.insert(path.to_owned(), (object_hash, object));

		Ok(object_hash)
	}

	async fn cache_object_for_dependency(
		&mut self,
		path: &Path,
		_metadata: &Metadata,
		dependency: Dependency,
	) -> Result<ObjectHash> {
		// Create the object and add it to the cache.
		let object = Object::Dependency(dependency);
		let object_hash = object.hash();
		self.cache.insert(path.to_owned(), (object_hash, object));

		Ok(object_hash)
	}
}
