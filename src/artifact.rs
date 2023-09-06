use crate::{directory, file, return_error, symlink, Client, Result};

crate::id!();

/// An artifact handle.
#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

#[derive(Clone, Debug)]
pub enum Value {
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
	pub fn value(&self) -> Value {
		match self.0.kind() {
			crate::Kind::Directory => Value::Directory(self.0.clone().try_into().unwrap()),
			crate::Kind::File => Value::File(self.0.clone().try_into().unwrap()),
			crate::Kind::Symlink => Value::Symlink(self.0.clone().try_into().unwrap()),
			_ => unreachable!(),
		}
	}

	#[must_use]
	pub fn as_directory(&self) -> Option<directory::Handle> {
		match self.0.kind() {
			crate::Kind::Directory => Some(self.0.clone().try_into().unwrap()),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_file(&self) -> Option<file::Handle> {
		match self.0.kind() {
			crate::Kind::File => Some(self.0.clone().try_into().unwrap()),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_symlink(&self) -> Option<symlink::Handle> {
		match self.0.kind() {
			crate::Kind::Symlink => Some(self.0.clone().try_into().unwrap()),
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
			crate::Kind::Directory | crate::Kind::File | crate::Kind::Symlink => Ok(Self(value)),
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
			crate::Kind::Directory | crate::Kind::File | crate::Kind::Symlink => Ok(Self(value)),
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

// use super::Artifact;
// use crate::{error::Result, id::Id, server::Server};
// use async_recursion::async_recursion;
// use futures::stream::{FuturesUnordered, TryStreamExt};
// use std::collections::{HashSet, VecDeque};

// impl Artifact {
// 	/// Collect an artifact's references.
// 	#[async_recursion]
// 	pub async fn references(&self, tg: &Server) -> Result<HashSet<Id, fnv::FnvBuildHasher>> {
// 		match self {
// 			Artifact::Directory(directory) => directory.references(tg).await,
// 			Artifact::File(file) => file.references(tg).await,
// 			Artifact::Symlink(symlink) => Ok(symlink.references()),
// 		}
// 	}

// 	/// Collect an artifact's recursive references.
// 	pub async fn recursive_references(
// 		&self,
// 		tg: &Server,
// 	) -> Result<HashSet<Id, fnv::FnvBuildHasher>> {
// 		// Create a queue of artifacts and a set of futures.
// 		let mut references = HashSet::default();
// 		let mut queue = VecDeque::new();
// 		let mut futures = FuturesUnordered::new();
// 		queue.push_back(self.clone());

// 		while let Some(artifact) = queue.pop_front() {
// 			// Add a request for the artifact's references to the futures.
// 			futures.push(async move { artifact.references(tg).await });

// 			// If the queue is empty, then get more artifacts from the futures.
// 			if queue.is_empty() {
// 				// Get more artifacts from the futures.
// 				if let Some(artifacts) = futures.try_next().await? {
// 					// Handle each artifact.
// 					for artifact in artifacts {
// 						// Insert the artifact into the set of references.
// 						let inserted = references.insert(artifact.clone());

// 						// If the artifact was new, then add it to the queue.
// 						if inserted {
// 							queue.push_back(artifact);
// 						}
// 					}
// 				}
// 			}
// 		}

// 		Ok(references)
// 	}
// }
