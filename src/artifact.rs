use crate::{directory, file, id, return_error, symlink, Client, Error, Kind, Result};
use futures::stream::{FuturesUnordered, TryStreamExt};
use std::collections::{HashSet, VecDeque};

crate::id!();

/// An artifact handle.
#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

/// An artifact variant.
#[derive(Clone, Debug)]
pub enum Variant {
	/// A directory.
	Directory(directory::Handle),

	/// A file.
	File(file::Handle),

	/// A symlink.
	Symlink(symlink::Handle),
}

impl Handle {
	#[must_use]
	pub fn with_id(id: Id) -> Self {
		Self(crate::Handle::with_id(id.into()))
	}

	#[must_use]
	pub fn expect_id(&self) -> Id {
		self.0.expect_id().try_into().unwrap()
	}

	pub async fn id(&self, client: &Client) -> Result<Id> {
		Ok(self.0.id(client).await?.try_into().unwrap())
	}

	#[must_use]
	pub fn variant(&self) -> Variant {
		match self.0.kind() {
			Kind::Directory => Variant::Directory(self.0.clone().try_into().unwrap()),
			Kind::File => Variant::File(self.0.clone().try_into().unwrap()),
			Kind::Symlink => Variant::Symlink(self.0.clone().try_into().unwrap()),
			_ => unreachable!(),
		}
	}

	/// Collect an artifact's recursive references.
	pub async fn recursive_references(
		&self,
		client: &Client,
	) -> Result<HashSet<Id, id::BuildHasher>> {
		// Store the handle.
		self.0.store(client).await?;

		// Create a queue of artifacts and a set of futures.
		let mut references = HashSet::default();
		let mut queue = VecDeque::new();
		let mut futures = FuturesUnordered::new();
		queue.push_back(self.clone());

		while let Some(artifact) = queue.pop_front() {
			// Add a request for the artifact's references to the futures.
			futures.push(async move {
				Ok::<Vec<Handle>, Error>(match artifact.variant() {
					Variant::Directory(directory) => {
						directory.entries(client).await?.values().cloned().collect()
					},
					Variant::File(file) => file.references(client).await?.to_owned(),
					Variant::Symlink(symlink) => symlink
						.target(client)
						.await?
						.value(client)
						.await?
						.artifacts()
						.cloned()
						.collect(),
				})
			});

			// If the queue is empty, then get more artifacts from the futures.
			if queue.is_empty() {
				// Get more artifacts from the futures.
				if let Some(artifacts) = futures.try_next().await? {
					// Handle each artifact.
					for artifact in artifacts {
						// Insert the artifact into the set of references.
						let inserted = references.insert(artifact.expect_id());

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

	#[must_use]
	pub fn as_directory(&self) -> Option<directory::Handle> {
		match self.0.kind() {
			Kind::Directory => Some(self.0.clone().try_into().unwrap()),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_file(&self) -> Option<file::Handle> {
		match self.0.kind() {
			Kind::File => Some(self.0.clone().try_into().unwrap()),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_symlink(&self) -> Option<symlink::Handle> {
		match self.0.kind() {
			Kind::Symlink => Some(self.0.clone().try_into().unwrap()),
			_ => None,
		}
	}
}

impl From<Id> for crate::Id {
	fn from(value: Id) -> Self {
		value.0
	}
}

impl TryFrom<crate::Id> for Id {
	type Error = crate::Error;

	fn try_from(value: crate::Id) -> Result<Self, Self::Error> {
		match value.kind() {
			Kind::Directory | Kind::File | Kind::Symlink => Ok(Self(value)),
			_ => return_error!("Expected an artifact ID."),
		}
	}
}

impl From<Handle> for crate::Handle {
	fn from(value: Handle) -> Self {
		value.0
	}
}

impl TryFrom<crate::Handle> for Handle {
	type Error = crate::Error;

	fn try_from(value: crate::Handle) -> Result<Self, Self::Error> {
		match value.kind() {
			Kind::Directory | Kind::File | Kind::Symlink => Ok(Self(value)),
			_ => return_error!("Expected an artifact value."),
		}
	}
}

impl From<directory::Handle> for Handle {
	fn from(value: directory::Handle) -> Self {
		Self(value.into())
	}
}

impl From<file::Handle> for Handle {
	fn from(value: file::Handle) -> Self {
		Self(value.into())
	}
}

impl From<symlink::Handle> for Handle {
	fn from(value: symlink::Handle) -> Self {
		Self(value.into())
	}
}
