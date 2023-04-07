use super::Directory;
use crate::{
	artifact::Artifact,
	error::{return_error, Result, WrapErr},
	instance::Instance,
	path::{self, Path},
};
use async_recursion::async_recursion;
use std::collections::BTreeMap;

impl Directory {
	pub async fn builder(&self, tg: &Instance) -> Result<Builder> {
		Ok(Builder::with_entries(self.entries(tg).await?))
	}
}

#[derive(Clone, Debug, Default)]
pub struct Builder {
	entries: BTreeMap<String, Artifact>,
}

impl Builder {
	#[must_use]
	pub fn new() -> Self {
		Self::default()
	}

	#[must_use]
	pub fn with_entries(entries: BTreeMap<String, Artifact>) -> Self {
		Self { entries }
	}

	#[async_recursion]
	pub async fn add(
		mut self,
		tg: &Instance,
		path: impl Into<Path> + Send + 'async_recursion,
		artifact: impl Into<Artifact> + Send + 'async_recursion,
	) -> Result<Self> {
		// Get the path.
		let path = path.into();
		if let Some(path::Component::Parent) = path.components().first() {
			return_error!("Invalid path.");
		}

		// Get the artifact.
		let artifact = artifact.into();

		// Get the name of the first component.
		let name = path
			.components()
			.first()
			.wrap_err("Expected the path to have at least one component.")?
			.as_normal()
			.wrap_err("Expected the path component to be a normal component.")?;

		// Collect the trailing path.
		let trailing_path: Path = path.components().iter().skip(1).cloned().collect();

		let artifact = if trailing_path.components().is_empty() {
			artifact
		} else {
			// Get or create a child directory.
			let builder = if let Some(child) = self.entries.get(name) {
				child
					.as_directory()
					.wrap_err("Expected the artifact to be a directory.")?
					.builder(tg)
					.await?
			} else {
				Self::new()
			};

			// Recurse.
			builder
				.add(tg, trailing_path, artifact)
				.await?
				.build(tg)
				.await?
				.into()
		};

		// Add the artifact.
		self.entries.insert(name.to_owned(), artifact);

		Ok(self)
	}

	pub async fn build(self, tg: &Instance) -> Result<Directory> {
		Directory::new(tg, self.entries).await
	}
}
