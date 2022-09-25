use crate::{
	expression::{Dependency, Directory, Expression, File, Symlink},
	hash::{Hash, Hasher},
};
use anyhow::{anyhow, bail, Context, Result};
use async_recursion::async_recursion;
use camino::{Utf8Component, Utf8PathBuf};
use fnv::FnvBuildHasher;
use futures::future::try_join_all;
use std::{
	collections::HashMap,
	fs::Metadata,
	os::unix::prelude::PermissionsExt,
	path::Path,
	path::PathBuf,
	sync::{Arc, RwLock},
};

pub struct Cache {
	root_path: PathBuf,
	semaphore: Arc<tokio::sync::Semaphore>,
	cache: RwLock<HashMap<PathBuf, (Hash, Expression), FnvBuildHasher>>,
}

impl Cache {
	pub fn new(root_path: &Path, semaphore: Arc<tokio::sync::Semaphore>) -> Cache {
		let root_path = root_path.to_owned();
		let cache = RwLock::new(HashMap::default());
		Cache {
			root_path,
			semaphore,
			cache,
		}
	}

	pub async fn get(&self, path: &Path) -> Result<Option<(Hash, Expression)>> {
		// Fill the cache for this path if necessary.
		if self.cache.read().unwrap().get(path).is_none() {
			tracing::trace!(r#"Filling cache for path "{}"."#, path.display());

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
				self.cache_for_directory(path, &metadata)
					.await
					.with_context(|| {
						format!(r#"Failed to cache directory at path "{}"."#, path.display())
					})?;
			} else if metadata.is_file() {
				self.cache_for_file(path, &metadata)
					.await
					.with_context(|| {
						format!(r#"Failed to cache file at path "{}"."#, path.display())
					})?;
			} else if metadata.is_symlink() {
				self.cache_for_symlink(path, &metadata)
					.await
					.with_context(|| {
						format!(r#"Failed to cache symlink at path "{}"."#, path.display())
					})?;
			} else {
				bail!("The path must point to a directory, file, or symlink.");
			};
		}

		// Retrieve and return the entry.
		let entry = self.cache.read().unwrap().get(path).unwrap().clone();
		Ok(Some(entry))
	}

	#[async_recursion]
	async fn cache_for_directory(&self, path: &Path, _metadata: &Metadata) -> Result<()> {
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
		let entries = try_join_all(entry_names.into_iter().map(|entry_name| async {
			let entry_path = path.join(&entry_name);
			self.get(&entry_path).await?;
			let (hash, _) = self.cache.read().unwrap().get(&entry_path).unwrap().clone();
			Ok::<_, anyhow::Error>((entry_name, hash))
		}))
		.await?
		.into_iter()
		.collect();

		// Create the expression and add it to the cache.
		let expression = Expression::Directory(Directory { entries });
		self.add_expression(path, expression);

		Ok(())
	}

	async fn cache_for_file(&self, path: &Path, metadata: &Metadata) -> Result<()> {
		// Compute the file's blob hash.
		let permit = self.semaphore.acquire().await.unwrap();
		let mut file = tokio::fs::File::open(path).await?;
		let mut hasher = Hasher::new();
		tokio::io::copy(&mut file, &mut hasher).await?;
		let blob_hash = hasher.finalize();
		drop(file);
		drop(permit);

		// Determine if the file is executable.
		let executable = (metadata.permissions().mode() & 0o111) != 0;

		// Create the expression and add it to the cache.
		let expression = Expression::File(File {
			hash: blob_hash,
			executable,
		});
		self.add_expression(path, expression);

		Ok(())
	}

	async fn cache_for_symlink(&self, path: &Path, _metadata: &Metadata) -> Result<()> {
		// Read the symlink.
		let permit = self.semaphore.acquire().await.unwrap();
		let target = tokio::fs::read_link(path)
			.await
			.with_context(|| format!(r#"Failed to read symlink at path "{}"."#, path.display()))?;
		let target = Utf8PathBuf::from_path_buf(target)
			.map_err(|_| anyhow!("Symlink target is not a valid UTF-8 path."))?;
		drop(permit);

		// Determine if the symlink is a symlink or a dependency by checking if the target has enough leading parent directory components to point outside the root path.
		let path_in_root = path.strip_prefix(&self.root_path).unwrap();
		let path_depth_in_root = path_in_root.components().count();
		let target_leading_double_dot_count = target
			.components()
			.take_while(|component| matches!(component, Utf8Component::ParentDir))
			.count();
		let is_symlink = target_leading_double_dot_count < path_depth_in_root;

		// Create the expression and add it to the cache.
		let expression = if is_symlink {
			Expression::Symlink(Symlink { target })
		} else {
			// Parse the expression hash of the dependency artifact from the last path component of the target.
			let hash: Hash = target
				.components()
				.last()
				.ok_or_else(|| anyhow!("Invalid symlink."))?
				.as_str()
				.parse()
				.context("Failed to parse the last path component as a hash.")?;
			Expression::Dependency(Dependency { artifact: hash })
		};
		self.add_expression(path, expression);

		Ok(())
	}

	fn add_expression(&self, path: &Path, expression: Expression) {
		let data = serde_json::to_vec(&expression).unwrap();
		let hash = Hash::new(&data);
		self.cache
			.write()
			.unwrap()
			.insert(path.to_owned(), (hash, expression));
	}
}
