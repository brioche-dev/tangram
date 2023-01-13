use crate::{
	artifact::{Artifact, ArtifactHash, Dependency, Directory, File, Symlink},
	blob::BlobHash,
	hash::Hasher,
};
use anyhow::{anyhow, bail, Context, Result};
use async_recursion::async_recursion;
use camino::Utf8PathBuf;
use fnv::FnvBuildHasher;
use futures::future::try_join_all;
use std::{
	collections::HashMap,
	fs::Metadata,
	os::unix::prelude::PermissionsExt,
	path::{Path, PathBuf},
	sync::{Arc, RwLock},
};

pub struct Watcher {
	path: PathBuf,
	semaphore: Arc<tokio::sync::Semaphore>,
	cache: RwLock<HashMap<PathBuf, (ArtifactHash, Artifact), FnvBuildHasher>>,
}

impl Watcher {
	#[must_use]
	pub fn new(path: &Path, semaphore: Arc<tokio::sync::Semaphore>) -> Watcher {
		let path = path.to_owned();
		let cache = RwLock::new(HashMap::default());
		Watcher {
			path,
			semaphore,
			cache,
		}
	}

	pub async fn get(&self, path: &Path) -> Result<Option<(ArtifactHash, Artifact)>> {
		// Fill the cache for this path if necessary.
		if self.cache.read().unwrap().get(path).is_none() {
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
				self.cache_directory(path, &metadata)
					.await
					.with_context(|| {
						format!(r#"Failed to cache directory at path "{}"."#, path.display())
					})?;
			} else if metadata.is_file() {
				self.cache_file(path, &metadata).await.with_context(|| {
					format!(r#"Failed to cache file at path "{}"."#, path.display())
				})?;
			} else if metadata.is_symlink() {
				self.cache_symlink(path, &metadata).await.with_context(|| {
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
	async fn cache_directory(&self, path: &Path, _metadata: &Metadata) -> Result<()> {
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
				.context("All file names must be valid UTF-8.")?
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

		// Create the artifact.
		let artifact = Artifact::Directory(Directory { entries });

		// Add the artifact to the cache.
		self.cache
			.write()
			.unwrap()
			.insert(path.to_owned(), (artifact.hash(), artifact));

		Ok(())
	}

	async fn cache_file(&self, path: &Path, metadata: &Metadata) -> Result<()> {
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

		// Create the artifact.
		let artifact = Artifact::File(File {
			blob: blob_hash,
			executable,
		});

		// Add the artifact to the cache.
		self.cache
			.write()
			.unwrap()
			.insert(path.to_owned(), (artifact.hash(), artifact));

		Ok(())
	}

	async fn cache_symlink(&self, path: &Path, _metadata: &Metadata) -> Result<()> {
		// Read the symlink.
		let permit = self.semaphore.acquire().await.unwrap();
		let target = tokio::fs::read_link(path)
			.await
			.with_context(|| format!(r#"Failed to read symlink at path "{}"."#, path.display()))?;
		let target = Utf8PathBuf::from_path_buf(target)
			.map_err(|_| anyhow!("The symlink target is not a valid UTF-8 path."))?;
		drop(permit);

		// Create the artifact.
		let artifact = if target.is_absolute() {
			// A symlink that has an absolute target that points into the checkouts directory is a dependency.
			let target = target
				.strip_prefix(&self.path.join("checkouts"))
				.map_err(|_| anyhow!("Invalid symlink."))?;

			// Get the path components.
			let mut components = target.components().peekable();

			// Parse the hash from the first component.
			let artifact_hash: ArtifactHash = components
				.next()
				.context("Invalid symlink.")?
				.as_str()
				.parse()
				.context("Failed to parse the path component as a hash.")?;

			// Collect the remaining components to get the path within the dependency.
			let path = if components.peek().is_some() {
				Some(components.collect())
			} else {
				None
			};

			Artifact::Dependency(Dependency {
				artifact: artifact_hash,
				path,
			})
		} else {
			Artifact::Symlink(Symlink { target })
		};

		// Add the artifact to the cache.
		self.cache
			.write()
			.unwrap()
			.insert(path.to_owned(), (artifact.hash(), artifact));

		Ok(())
	}
}
