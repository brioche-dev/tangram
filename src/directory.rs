use crate as tg;
use crate::error::{Result, WrapErr};
use crate::{artifact, error::Error, instance::Instance, subpath};
use std::collections::BTreeMap;

#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
pub struct Directory {
	/// The directory's entries.
	#[tangram_serialize(id = 0)]
	pub entries: BTreeMap<String, tg::Artifact>,
}

crate::value!(Directory);

impl tg::Directory {
	#[must_use]
	pub fn new(entries: BTreeMap<String, tg::Artifact>) -> Self {
		Directory { entries }.into()
	}

	pub async fn builder(&self, tg: &Instance) -> Result<Builder> {
		Ok(Builder::new(self.get(tg).await?.entries.clone()))
	}

	pub async fn entries(&self, tg: &Instance) -> Result<&BTreeMap<String, tg::Artifact>, Error> {
		Ok(&self.get(tg).await?.entries)
	}
}

impl Directory {
	#[must_use]
	pub fn children(&self) -> Vec<tg::Value> {
		self.entries
			.values()
			.map(|child| child.clone().into())
			.collect()
	}
}

impl tg::Directory {
	pub async fn get_entry(&self, tg: &Instance, path: &tg::Subpath) -> Result<tg::Artifact> {
		let artifact = self
			.try_get_entry(tg, path)
			.await?
			.wrap_err("Failed to get the artifact.")?;
		Ok(artifact)
	}

	pub async fn try_get_entry(
		&self,
		tg: &Instance,
		path: &tg::Subpath,
	) -> Result<Option<tg::Artifact>> {
		// Track the current artifact.
		let mut artifact = self.clone();

		// Track the current subpath.
		let mut current_subpath = subpath::Subpath::empty();

		// Handle each path component.
		for name in path.components(tg).await? {
			// The artifact must be a directory.
			let Some(directory) = artifact.as_directory() else {
				return Ok(None);
			};

			// Update the current subpath.
			current_subpath = current_subpath.join(name.parse().unwrap());

			// Get the entry. If it doesn't exist, return `None`.
			let Some(id) = directory.entries.get(name).cloned() else {
				return Ok(None);
			};

			// Get the artifact.
			artifact = tg::Artifact::with_id(id);

			// If the artifact is a symlink, then resolve it.
			if let Artifact::Symlink(symlink) = &artifact {
				match symlink
					.resolve_from(tg, Some(symlink))
					.await
					.wrap_err("Failed to resolve the symlink.")?
				{
					Some(resolved) => artifact = resolved,
					None => return Ok(None),
				}
			}
		}

		Ok(Some(artifact))
	}
}

#[derive(Clone, Debug, Default)]
pub struct Builder {
	entries: BTreeMap<String, tg::Artifact>,
}

impl Builder {
	#[must_use]
	pub fn new(entries: BTreeMap<String, tg::Artifact>) -> Self {
		Self { entries }
	}

	// #[async_recursion]
	// pub async fn add(
	// 	mut self,
	// 	tg: &Instance,
	// 	path: &Subpath,
	// 	artifact: impl Into<Artifact> + Send + 'async_recursion,
	// ) -> Result<Self> {
	// 	// Get the artifact.
	// 	let artifact = artifact.into();

	// 	// Get the first component.
	// 	let name = path
	// 		.components()
	// 		.first()
	// 		.wrap_err("Expected the path to have at least one component.")?;

	// 	// Collect the trailing path.
	// 	let trailing_path: Subpath = path.components().iter().skip(1).cloned().collect();

	// 	let artifact = if trailing_path.components().is_empty() {
	// 		artifact
	// 	} else {
	// 		// Get or create a child directory.
	// 		let builder = if let Some(child) = self.entries.get(name) {
	// 			child
	// 				.as_directory()
	// 				.wrap_err("Expected the artifact to be a directory.")?
	// 				.builder(tg)
	// 				.await?
	// 		} else {
	// 			Self::new()
	// 		};

	// 		// Recurse.
	// 		builder
	// 			.add(tg, &trailing_path, artifact)
	// 			.await?
	// 			.build()
	// 			.into()
	// 	};

	// 	// Add the artifact.
	// 	self.entries.insert(name.clone(), artifact);

	// 	Ok(self)
	// }

	// #[async_recursion]
	// pub async fn remove(mut self, tg: &Instance, path: &Subpath) -> Result<Self> {
	// 	// Get the first component.
	// 	let name = path
	// 		.components()
	// 		.first()
	// 		.wrap_err("Expected the path to have at least one component.")?;

	// 	// Collect the trailing path.
	// 	let trailing_path: Subpath = path.components().iter().skip(1).cloned().collect();

	// 	if trailing_path.components().is_empty() {
	// 		// Remove the entry.
	// 		self.entries.remove(name);
	// 	} else {
	// 		// Get a child directory.
	// 		let builder = if let Some(child) = self.entries.get(name) {
	// 			child
	// 				.as_directory()
	// 				.wrap_err("Expected the artifact to be a directory.")?
	// 				.builder(tg)
	// 				.await?
	// 		} else {
	// 			return Err(crate::error!("The path does not exist."));
	// 		};

	// 		// Recurse.
	// 		let artifact = builder.remove(tg, &trailing_path).await?.build().into();

	// 		// Add the new artifact.
	// 		self.entries.insert(name.clone(), artifact);
	// 	};

	// 	Ok(self)
	// }

	#[must_use]
	pub fn build(self) -> tg::Directory {
		tg::Directory::new(self.entries)
	}
}
