use crate::{
	artifact::{self, Artifact},
	error::{Context, Result},
	path::Path,
	Instance,
};
use async_recursion::async_recursion;
use std::{collections::BTreeMap, sync::Arc};

#[derive(
	Clone,
	Debug,
	Default,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Directory {
	#[buffalo(id = 0)]
	pub entries: BTreeMap<String, artifact::Hash>,
}

impl Directory {
	/// Create a new directory.
	#[must_use]
	pub fn new() -> Directory {
		Directory::default()
	}
}

impl Directory {
	#[allow(clippy::unused_async)]
	#[async_recursion]
	pub async fn add(
		&mut self,
		tg: &Arc<Instance>,
		path: &Path,
		artifact_hash: artifact::Hash,
	) -> Result<()> {
		// Get the name of the first component.
		let name = path
			.components
			.first()
			.context("Expected the path to have at least one component.")?
			.as_normal()
			.context("Expected the path component to be a normal component.")?;

		// Collect the trailing path.
		let trailing_path: Path = path.components.iter().skip(1).cloned().collect();

		let artifact_hash = if trailing_path.components.is_empty() {
			artifact_hash
		} else {
			// Get or create a child directory.
			let mut child = if let Some(child_hash) = self.entries.get(name) {
				tg.get_artifact_local(*child_hash)?
					.into_directory()
					.context("Expected the existing entry to be a directory.")?
			} else {
				Directory::new()
			};

			// Recurse.
			child.add(tg, &trailing_path, artifact_hash).await?;

			// Add this artifact.
			tg.add_artifact(&Artifact::Directory(child)).await?
		};

		// Add the artifact.
		self.entries.insert(name.to_owned(), artifact_hash);

		Ok(())
	}

	#[allow(clippy::unused_async)]
	pub async fn get(&self, tg: &Instance, path: &Path) -> Result<Artifact> {
		let mut artifact = Artifact::Directory(self.clone());
		for component in &path.components {
			let name = component
				.as_normal()
				.context("Expected the path component to be normal.")?;
			let artifact_hash = artifact
				.as_directory()
				.context("Expected a directory.")?
				.entries
				.get(name)
				.copied()
				.with_context(|| format!(r#"Failed to find the child at path "{path}"."#))?;
			artifact = tg
				.get_artifact_local(artifact_hash)
				.context("Failed to get the artifact.")?;
		}
		Ok(artifact)
	}
}
