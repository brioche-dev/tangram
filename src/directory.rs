use crate::{
	artifact,
	subpath::{self, Subpath},
	Client, Error, Result, WrapErr,
};
use std::collections::BTreeMap;

crate::id!();

crate::kind!(Directory);

#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

#[derive(Clone, Debug)]
pub struct Value {
	/// The directory's entries.
	pub entries: BTreeMap<String, artifact::Handle>,
}

#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
pub struct Data {
	/// The directory's entries.
	#[tangram_serialize(id = 0)]
	pub entries: BTreeMap<String, crate::artifact::Id>,
}

impl Handle {
	#[must_use]
	pub fn new(entries: BTreeMap<String, artifact::Handle>) -> Self {
		Self::with_value(Value { entries })
	}

	pub async fn builder(&self, tg: &Client) -> Result<Builder> {
		Ok(Builder::new(self.value(tg).await?.entries.clone()))
	}

	pub async fn entries(&self, tg: &Client) -> Result<&BTreeMap<String, artifact::Handle>, Error> {
		Ok(&self.value(tg).await?.entries)
	}

	pub async fn get(&self, tg: &Client, path: &Subpath) -> Result<artifact::Handle> {
		let artifact = self
			.try_get(tg, path)
			.await?
			.wrap_err("Failed to get the artifact.")?;
		Ok(artifact)
	}

	pub async fn try_get(&self, tg: &Client, path: &Subpath) -> Result<Option<artifact::Handle>> {
		// Track the current artifact.
		let mut artifact: artifact::Handle = self.clone().into();

		// Track the current subpath.
		let mut current_subpath = subpath::Subpath::empty();

		// Handle each path component.
		for name in path.components() {
			// The artifact must be a directory.
			let Some(directory) = artifact.as_directory() else {
				return Ok(None);
			};

			// Update the current subpath.
			current_subpath = current_subpath.join(name.parse().unwrap());

			// Get the entry. If it doesn't exist, return `None`.
			let Some(entry) = directory.entries(tg).await?.get(name).cloned() else {
				return Ok(None);
			};

			// Get the artifact.
			artifact = entry;

			// If the artifact is a symlink, then resolve it.
			if let artifact::Value::Symlink(symlink) = &artifact.value() {
				match symlink
					.resolve_from(tg, Some(symlink.value(tg).await?))
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

impl Value {
	#[must_use]
	pub fn from_data(data: Data) -> Self {
		let entries = data
			.entries
			.into_iter()
			.map(|(name, id)| (name, artifact::Handle::with_id(id)))
			.collect();
		Value { entries }
	}

	#[must_use]
	pub fn to_data(&self) -> Data {
		todo!()
	}

	#[must_use]
	pub fn children(&self) -> Vec<crate::Handle> {
		self.entries
			.values()
			.map(|child| child.clone().into())
			.collect()
	}
}

impl Data {
	#[must_use]
	pub fn children(&self) -> Vec<crate::Id> {
		self.entries.values().copied().map(Into::into).collect()
	}
}

#[derive(Clone, Debug, Default)]
pub struct Builder {
	entries: BTreeMap<String, artifact::Handle>,
}

impl Builder {
	#[must_use]
	pub fn new(entries: BTreeMap<String, artifact::Handle>) -> Self {
		Self { entries }
	}

	// #[async_recursion]
	// pub async fn add(
	// 	mut self,
	// 	tg: &Server,
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
	// pub async fn remove(mut self, tg: &Server, path: &Subpath) -> Result<Self> {
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
	pub fn build(self) -> Handle {
		Handle::new(self.entries)
	}
}
