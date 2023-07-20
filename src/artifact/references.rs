use super::Artifact;
use crate::{error::Result, instance::Instance};
use async_recursion::async_recursion;
use futures::stream::{FuturesUnordered, TryStreamExt};
use std::collections::{HashSet, VecDeque};

impl Artifact {
	/// Collect an artifact's references.
	#[async_recursion]
	pub async fn references(&self, tg: &Instance) -> Result<Vec<Artifact>> {
		match self {
			Artifact::Directory(directory) => directory.references(tg).await,
			Artifact::File(file) => file.references(tg).await,
			Artifact::Symlink(symlink) => Ok(symlink.references()),
		}
	}

	/// Collect an artifact's recursive references.
	pub async fn recursive_references(
		&self,
		tg: &Instance,
	) -> Result<HashSet<Artifact, fnv::FnvBuildHasher>> {
		// Create a queue of artifacts and a set of futures.
		let mut references = HashSet::default();
		let mut queue = VecDeque::new();
		let mut futures = FuturesUnordered::new();
		queue.push_back(self.clone());

		while let Some(artifact) = queue.pop_front() {
			// Add a request for the artifact's references to the futures.
			futures.push(async move { artifact.references(tg).await });

			// If the queue is empty, then get more artifacts from the futures.
			if queue.is_empty() {
				// Get more artifacts from the futures.
				if let Some(artifacts) = futures.try_next().await? {
					// Handle each artifact.
					for artifact in artifacts {
						// Insert the artifact into the set of references.
						let inserted = references.insert(artifact.clone());

						// If the artifact was new, then add it to the queue.
						if inserted {
							queue.push_back(artifact);
						}
					}
				}
			}
		}

		Ok(references)
	}
}
