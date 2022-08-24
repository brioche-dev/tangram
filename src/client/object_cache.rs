use crate::{
	artifact::Artifact,
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
	pub root_path: PathBuf,
	pub semaphore: Arc<tokio::sync::Semaphore>,
	pub cache: IndexMap<PathBuf, (ObjectHash, Object)>,
}

impl ObjectCache {
	pub fn new(root_path: &Path, semaphore: Arc<tokio::sync::Semaphore>) -> ObjectCache {
		let root_path = root_path.to_owned();
		let cache = IndexMap::new();
		ObjectCache {
			root_path,
			semaphore,
			cache,
		}
	}

	pub async fn get(&mut self, path: &Path) -> Result<Option<&(ObjectHash, Object)>> {
		// Fill the cache for this path if necessary.
		if self.cache.get(path).is_none() {
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
				self.cache_object_for_symlink(path, &metadata).await?;
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

		// Determine if the symlink is a dependency by checking if the target points outside the root path.
		let canonicalized_target = tokio::fs::canonicalize(self.root_path.join(&target)).await?;
		let is_dependency = !canonicalized_target.starts_with(&self.root_path);

		// Create the object and add it to the cache.
		let object_hash = if !is_dependency {
			let object = Object::Symlink(Symlink { target });
			let object_hash = object.hash();
			self.cache.insert(path.to_owned(), (object_hash, object));
			object_hash
		} else {
			// Parse the artifact from the target.
			let artifact: Artifact = target
				.components()
				.last()
				.ok_or_else(|| anyhow!("Invalid symlink."))?
				.as_str()
				.parse()?;
			let object = Object::Dependency(Dependency { artifact });
			let object_hash = object.hash();
			self.cache.insert(path.to_owned(), (object_hash, object));
			object_hash
		};

		Ok(object_hash)
	}
}
