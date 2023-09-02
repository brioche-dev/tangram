use crate::error::Result;
use crate::{self as tg, Id};
use crate::{error::return_error, Kind};

// mod bundle;
// pub mod checkin;
// mod checkout;
// mod references;

/// An artifact.
#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
#[tangram_serialize(into = "tg::Value", try_from = "tg::Value")]
pub struct Value(tg::Value);

impl std::ops::Deref for Value {
	type Target = tg::Value;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

#[derive(Clone, Debug)]
pub enum Artifact {
	/// A directory.
	Directory(tg::Directory),

	/// A file.
	File(tg::File),

	/// A symlink.
	Symlink(tg::Symlink),
}

impl From<Value> for tg::Value {
	fn from(value: Value) -> Self {
		value.0
	}
}

impl TryFrom<tg::Value> for Value {
	type Error = crate::error::Error;

	fn try_from(value: tg::Value) -> std::result::Result<Self, Self::Error> {
		match value.kind() {
			Kind::Directory | Kind::File | Kind::Symlink => Ok(Self(value)),
			_ => return_error!("Expected an artifact value."),
		}
	}
}

impl Value {
	pub fn with_id(id: Id) -> Result<Self> {
		tg::Value::with_id(id).try_into()
	}

	#[must_use]
	pub fn get(&self) -> Artifact {
		match self.0.kind() {
			Kind::Directory => Artifact::Directory(self.0.clone().try_into().unwrap()),
			Kind::File => Artifact::File(self.0.clone().try_into().unwrap()),
			Kind::Symlink => Artifact::Symlink(self.0.clone().try_into().unwrap()),
			_ => unreachable!(),
		}
	}
}

impl Value {
	#[must_use]
	pub fn as_directory(&self) -> Option<tg::Directory> {
		match self.0.kind() {
			Kind::Directory => Some(self.0.clone().try_into().unwrap()),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_file(&self) -> Option<tg::File> {
		match self.0.kind() {
			Kind::File => Some(self.0.clone().try_into().unwrap()),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_symlink(&self) -> Option<tg::Symlink> {
		match self.0.kind() {
			Kind::Symlink => Some(self.0.clone().try_into().unwrap()),
			_ => None,
		}
	}
}

impl From<tg::Directory> for Value {
	fn from(value: tg::Directory) -> Self {
		Self(value.into())
	}
}

impl From<tg::File> for Value {
	fn from(value: tg::File) -> Self {
		Self(value.into())
	}
}

impl From<tg::Symlink> for Value {
	fn from(value: tg::Symlink) -> Self {
		Self(value.into())
	}
}

// use super::Artifact;
// use crate::{error::Result, id::Id, instance::Instance};
// use async_recursion::async_recursion;
// use futures::stream::{FuturesUnordered, TryStreamExt};
// use std::collections::{HashSet, VecDeque};

// impl Artifact {
// 	/// Collect an artifact's references.
// 	#[async_recursion]
// 	pub async fn references(&self, tg: &Instance) -> Result<HashSet<Id, fnv::FnvBuildHasher>> {
// 		match self {
// 			Artifact::Directory(directory) => directory.references(tg).await,
// 			Artifact::File(file) => file.references(tg).await,
// 			Artifact::Symlink(symlink) => Ok(symlink.references()),
// 		}
// 	}

// 	/// Collect an artifact's recursive references.
// 	pub async fn recursive_references(
// 		&self,
// 		tg: &Instance,
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
