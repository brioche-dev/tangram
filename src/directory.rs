use crate::{artifact, error, id, object, Artifact, Client, Error, Result, Subpath, WrapErr};
use async_recursion::async_recursion;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Directory(Handle);

crate::object!(Directory);

#[derive(Clone, Debug)]
pub(crate) struct Object {
	/// The directory's entries.
	pub entries: BTreeMap<String, Artifact>,
}

#[derive(
	Clone,
	Debug,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
pub(crate) struct Data {
	/// The directory's entries.
	#[tangram_serialize(id = 0)]
	pub entries: BTreeMap<String, artifact::Id>,
}

impl Directory {
	#[must_use]
	pub fn handle(&self) -> &Handle {
		&self.0
	}

	#[must_use]
	pub fn with_id(id: Id) -> Self {
		Self(Handle::with_id(id))
	}

	#[must_use]
	pub fn new(entries: BTreeMap<String, Artifact>) -> Self {
		Self(Handle::with_object(Object { entries }))
	}

	pub async fn builder(&self, client: &Client) -> Result<Builder> {
		Ok(Builder::new(self.0.object(client).await?.entries.clone()))
	}

	pub async fn entries(&self, client: &Client) -> Result<&BTreeMap<String, Artifact>, Error> {
		Ok(&self.0.object(client).await?.entries)
	}

	pub async fn get(&self, client: &Client, path: &Subpath) -> Result<Artifact> {
		let artifact = self
			.try_get(client, path)
			.await?
			.wrap_err("Failed to get the artifact.")?;
		Ok(artifact)
	}

	pub async fn try_get(&self, client: &Client, path: &Subpath) -> Result<Option<Artifact>> {
		// Track the current artifact.
		let mut artifact: Artifact = self.clone().into();

		// Track the current subpath.
		let mut current_subpath = Subpath::empty();

		// Handle each path component.
		for name in path.components() {
			// The artifact must be a directory.
			let Some(directory) = artifact.as_directory() else {
				return Ok(None);
			};

			// Update the current subpath.
			current_subpath = current_subpath.join(name.parse().unwrap());

			// Get the entry. If it doesn't exist, return `None`.
			let Some(entry) = directory.entries(client).await?.get(name).cloned() else {
				return Ok(None);
			};

			// Get the artifact.
			artifact = entry;

			// If the artifact is a symlink, then resolve it.
			if let Artifact::Symlink(symlink) = &artifact {
				match symlink
					.resolve_from(client, Some(symlink.clone().into()))
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

impl Id {
	#[must_use]
	pub fn with_data_bytes(bytes: &[u8]) -> Self {
		Self(crate::Id::new_hashed(id::Kind::Directory, bytes))
	}
}

impl Object {
	#[must_use]
	pub(crate) fn to_data(&self) -> Data {
		let entries = self
			.entries
			.iter()
			.map(|(name, artifact)| (name.clone(), artifact.expect_id()))
			.collect();
		Data { entries }
	}

	#[must_use]
	pub(crate) fn from_data(data: Data) -> Self {
		let entries = data
			.entries
			.into_iter()
			.map(|(name, id)| (name, Artifact::with_id(id)))
			.collect();
		Self { entries }
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Handle> {
		self.entries
			.values()
			.map(|child| child.clone().into())
			.collect()
	}
}

impl Data {
	#[must_use]
	pub fn children(&self) -> Vec<object::Id> {
		self.entries.values().copied().map(Into::into).collect()
	}
}

#[derive(Clone, Debug, Default)]
pub struct Builder {
	entries: BTreeMap<String, Artifact>,
}

impl Builder {
	#[must_use]
	pub fn new(entries: BTreeMap<String, Artifact>) -> Self {
		Self { entries }
	}

	#[async_recursion]
	pub async fn add(
		mut self,
		client: &Client,
		path: &Subpath,
		artifact: Artifact,
	) -> Result<Self> {
		// Get the first component.
		let name = path
			.components()
			.first()
			.wrap_err("Expected the path to have at least one component.")?;

		// Collect the trailing path.
		let trailing_path: Subpath = path.components().iter().skip(1).cloned().collect();

		let artifact = if trailing_path.components().is_empty() {
			artifact
		} else {
			// Get or create a child directory.
			let builder = if let Some(child) = self.entries.get(name) {
				child
					.as_directory()
					.wrap_err("Expected the artifact to be a directory.")?
					.builder(client)
					.await?
			} else {
				Self::default()
			};

			// Recurse.
			builder
				.add(client, &trailing_path, artifact)
				.await?
				.build()
				.into()
		};

		// Add the artifact.
		self.entries.insert(name.clone(), artifact);

		Ok(self)
	}

	#[async_recursion]
	pub async fn remove(mut self, client: &Client, path: &Subpath) -> Result<Self> {
		// Get the first component.
		let name = path
			.components()
			.first()
			.wrap_err("Expected the path to have at least one component.")?;

		// Collect the trailing path.
		let trailing_path: Subpath = path.components().iter().skip(1).cloned().collect();

		if trailing_path.components().is_empty() {
			// Remove the entry.
			self.entries.remove(name);
		} else {
			// Get a child directory.
			let builder = if let Some(child) = self.entries.get(name) {
				child
					.as_directory()
					.wrap_err("Expected the artifact to be a directory.")?
					.builder(client)
					.await?
			} else {
				return Err(error!("The path does not exist."));
			};

			// Recurse.
			let artifact = builder.remove(client, &trailing_path).await?.build().into();

			// Add the new artifact.
			self.entries.insert(name.clone(), artifact);
		};

		Ok(self)
	}

	#[must_use]
	pub fn build(self) -> Directory {
		Directory::new(self.entries)
	}
}
