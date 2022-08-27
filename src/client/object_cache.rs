use crate::{
	artifact::Artifact,
	hash::Hasher,
	object::{BlobHash, Dependency, Directory, File, Object, ObjectHash, Symlink},
};
use anyhow::{anyhow, bail, Context, Result};
use async_recursion::async_recursion;
use camino::{Utf8Component, Utf8PathBuf};
use fnv::FnvHashMap;
use std::{
	fs::Metadata,
	os::unix::prelude::PermissionsExt,
	path::Path,
	path::PathBuf,
	sync::{Arc, RwLock},
};

pub struct ObjectCache {
	root_path: PathBuf,
	semaphore: Arc<tokio::sync::Semaphore>,
	cache: RwLock<FnvHashMap<PathBuf, (ObjectHash, Object)>>,
}

impl ObjectCache {
	pub fn new(root_path: &Path, semaphore: Arc<tokio::sync::Semaphore>) -> ObjectCache {
		let root_path = root_path.to_owned();
		let cache = RwLock::new(FnvHashMap::default());
		ObjectCache {
			root_path,
			semaphore,
			cache,
		}
	}

	pub async fn get(&self, path: &Path) -> Result<Option<(ObjectHash, Object)>> {
		// Fill the cache for this path if necessary.
		if self.cache.read().unwrap().get(path).is_none() {
			tracing::trace!(
				r#"Object cache filling cache for path "{}"."#,
				path.display()
			);

			// Get the metadata for the path.
			let permit = self.semaphore.acquire().await.unwrap();
			let metadata = match tokio::fs::symlink_metadata(path).await {
				Ok(metadata) => metadata,
				Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
					return Ok(None);
				},
				Err(error) => {
					return Err(error.into());
				},
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
		let entry = self.cache.read().unwrap().get(path).unwrap().clone();
		Ok(Some(entry))
	}

	#[async_recursion]
	async fn cache_object_for_directory(&self, path: &Path, _metadata: &Metadata) -> Result<()> {
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
		drop(read_dir);
		drop(permit);

		// Recurse into the directory's entries.
		let entries = futures::future::try_join_all(entry_names.into_iter().map(|entry_name| {
			async {
				let entry_path = path.join(&entry_name);
				self.get(&entry_path).await?;
				let (object_hash, _) = self.cache.read().unwrap().get(&entry_path).unwrap().clone();
				Ok::<_, anyhow::Error>((entry_name, object_hash))
			}
		}))
		.await?
		.into_iter()
		.collect();

		let object = Object::Directory(Directory { entries });
		let object_hash = object.hash();
		self.cache
			.write()
			.unwrap()
			.insert(path.to_owned(), (object_hash, object));

		Ok(())
	}

	async fn cache_object_for_file(&self, path: &Path, metadata: &Metadata) -> Result<ObjectHash> {
		// Compute the file's blob hash.
		let permit = self.semaphore.acquire().await.unwrap();
		let mut file = tokio::fs::File::open(path).await?;
		let mut hasher = Hasher::new();
		tokio::io::copy(&mut file, &mut hasher).await?;
		let blob_hash = BlobHash(hasher.finalize());
		drop(file);
		drop(permit);

		// Determine if the file is executable.
		let executable = (metadata.permissions().mode() & 0o111) != 0;

		let object = Object::File(File {
			blob_hash,
			executable,
		});
		let object_hash = object.hash();
		self.cache
			.write()
			.unwrap()
			.insert(path.to_owned(), (object_hash, object));

		Ok(object_hash)
	}

	async fn cache_object_for_symlink(
		&self,
		path: &Path,
		_metadata: &Metadata,
	) -> Result<ObjectHash> {
		// Read the symlink.
		let permit = self.semaphore.acquire().await.unwrap();
		let target = tokio::fs::read_link(path).await?;
		let target = Utf8PathBuf::from_path_buf(target)
			.map_err(|_| anyhow!("Symlink target is not a valid UTF-8 path."))?;
		drop(permit);

		// Determine if the symlink is a dependency by checking if the target has enough leading parent dir components to escape the root path.
		let path_in_root = path.strip_prefix(&self.root_path).unwrap();
		let path_depth_in_root = path_in_root.components().count();
		let target_leading_double_dot_count = target
			.components()
			.take_while(|component| matches!(component, Utf8Component::ParentDir))
			.count();
		let is_dependency = target_leading_double_dot_count >= path_depth_in_root - 1;

		let object = if !is_dependency {
			Object::Symlink(Symlink { target })
		} else {
			// Parse the artifact from the target.
			let artifact: Artifact = target
				.components()
				.last()
				.ok_or_else(|| anyhow!("Invalid symlink."))?
				.as_str()
				.parse()?;
			Object::Dependency(Dependency { artifact })
		};

		let object_hash = object.hash();
		self.cache
			.write()
			.unwrap()
			.insert(path.to_owned(), (object_hash, object));

		Ok(object_hash)
	}
}
