use crate::{
	artifact::{self, Artifact},
	path::Path,
	Cli,
};
use anyhow::{Context, Result};
use async_recursion::async_recursion;
use std::collections::BTreeMap;

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

impl Cli {
	#[allow(clippy::unused_async)]
	#[async_recursion]
	pub async fn directory_add(
		&self,
		directory: &mut Directory,
		path: &Path,
		artifact_hash: artifact::Hash,
	) -> Result<()> {
		// Get the name of the first component.
		let name = path
			.components
			.first()
			.context("Expected the path to have at least one component.")?
			.as_normal()
			.context("Expected the path to have all normal components.")?;

		// Collect the trailing path.
		let trailing_path: Path = path.components.iter().skip(1).cloned().collect();

		let artifact_hash = if trailing_path.components.is_empty() {
			artifact_hash
		} else {
			// Get or create a child directory.
			let mut child = if let Some(child_hash) = directory.entries.get(name) {
				self.get_artifact_local(*child_hash)?
					.into_directory()
					.context("Expected the existing entry to be a directory.")?
			} else {
				Directory::new()
			};
			self.directory_add(&mut child, &trailing_path, artifact_hash)
				.await?;
			self.add_artifact(&Artifact::Directory(child)).await?
		};

		// Add the artifact.
		directory.entries.insert(name.to_owned(), artifact_hash);

		Ok(())
	}

	#[allow(clippy::unused_async)]
	pub async fn directory_get(
		&self,
		mut artifact_hash: artifact::Hash,
		path: &Path,
	) -> Result<artifact::Hash> {
		for component in &path.components {
			let name = component
				.as_normal()
				.context("Expected the path to have all normal components.")?;
			artifact_hash = self
				.get_artifact_local(artifact_hash)?
				.into_directory()
				.context("Expected a directory.")?
				.entries
				.get(name)
				.copied()
				.with_context(|| format!(r#"Failed to find the child at path "{path}"."#))?;
		}
		Ok(artifact_hash)
	}
}
