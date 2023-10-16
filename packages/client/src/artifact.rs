use crate::{
	checksum, directory, file, id, object, return_error, symlink, Checksum, Client, Directory,
	Error, File, Result, Symlink, Value,
};
use derive_more::{From, TryUnwrap};
use futures::stream::{FuturesUnordered, TryStreamExt};
use std::{
	collections::{HashSet, VecDeque},
	str::FromStr,
};

/// An artifact kind.
#[derive(Clone, Copy, Debug)]
pub enum Kind {
	Directory,
	File,
	Symlink,
}

/// An artifact ID.
#[derive(
	Clone,
	Copy,
	Debug,
	Eq,
	From,
	PartialEq,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(into = "crate::Id", try_from = "crate::Id")]
#[tangram_serialize(into = "crate::Id", try_from = "crate::Id")]
pub enum Id {
	/// A directory ID.
	Directory(directory::Id),

	/// A file ID.
	File(file::Id),

	/// A symlink ID.
	Symlink(symlink::Id),
}

/// An artifact.
#[derive(Clone, Debug, From, TryUnwrap)]
#[try_unwrap(ref)]
pub enum Artifact {
	/// A directory.
	Directory(Directory),

	/// A file.
	File(File),

	/// A symlink.
	Symlink(Symlink),
}

impl Artifact {
	#[must_use]
	pub fn with_id(id: Id) -> Self {
		match id {
			Id::Directory(id) => Self::Directory(Directory::with_id(id)),
			Id::File(id) => Self::File(File::with_id(id)),
			Id::Symlink(id) => Self::Symlink(Symlink::with_id(id)),
		}
	}

	pub async fn id(&self, client: &dyn Client) -> Result<Id> {
		match self {
			Self::Directory(directory) => Ok(directory.id(client).await?.into()),
			Self::File(file) => Ok(file.id(client).await?.into()),
			Self::Symlink(symlink) => Ok(symlink.id(client).await?.into()),
		}
	}

	#[must_use]
	pub fn expect_id(&self) -> Id {
		match self {
			Self::Directory(directory) => directory.expect_id().into(),
			Self::File(file) => file.expect_id().into(),
			Self::Symlink(symlink) => symlink.expect_id().into(),
		}
	}

	#[must_use]
	pub fn handle(&self) -> &object::Handle {
		match self {
			Self::Directory(directory) => directory.handle(),
			Self::File(file) => file.handle(),
			Self::Symlink(symlink) => symlink.handle(),
		}
	}

	/// Compute an artifact's checksum.
	#[allow(clippy::unused_async)]
	pub async fn checksum(
		&self,
		_client: &dyn Client,
		_algorithm: checksum::Algorithm,
	) -> Result<Checksum> {
		unimplemented!()
	}

	/// Collect an artifact's recursive references.
	pub async fn recursive_references(
		&self,
		client: &dyn Client,
	) -> Result<HashSet<Id, id::BuildHasher>> {
		// Create a queue of artifacts and a set of futures.
		let mut references = HashSet::default();
		let mut queue = VecDeque::new();
		let mut futures = FuturesUnordered::new();
		queue.push_back(self.clone());

		while let Some(artifact) = queue.pop_front() {
			// Add a request for the artifact's references to the futures.
			futures.push(async move {
				Ok::<Vec<Artifact>, Error>(match artifact {
					Self::Directory(directory) => {
						directory.entries(client).await?.values().cloned().collect()
					},
					Self::File(file) => file.references(client).await?.to_owned(),
					Self::Symlink(symlink) => {
						symlink.target(client).await?.artifacts().cloned().collect()
					},
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
}

impl std::fmt::Display for Id {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Directory(id) => write!(f, "{id}"),
			Self::File(id) => write!(f, "{id}"),
			Self::Symlink(id) => write!(f, "{id}"),
		}
	}
}

impl FromStr for Id {
	type Err = Error;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		crate::Id::from_str(s)?.try_into()
	}
}

impl From<Id> for crate::Id {
	fn from(value: Id) -> Self {
		match value {
			Id::Directory(id) => id.into(),
			Id::File(id) => id.into(),
			Id::Symlink(id) => id.into(),
		}
	}
}

impl TryFrom<crate::Id> for Id {
	type Error = Error;

	fn try_from(value: crate::Id) -> Result<Self, Self::Error> {
		match value.kind() {
			id::Kind::Directory => Ok(Self::Directory(value.try_into()?)),
			id::Kind::File => Ok(Self::File(value.try_into()?)),
			id::Kind::Symlink => Ok(Self::Symlink(value.try_into()?)),
			_ => return_error!("Expected an artifact ID."),
		}
	}
}

impl std::hash::Hash for Id {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		match self {
			Id::Directory(id) => std::hash::Hash::hash(id, state),
			Id::File(id) => std::hash::Hash::hash(id, state),
			Id::Symlink(id) => std::hash::Hash::hash(id, state),
		}
	}
}

impl From<Artifact> for object::Handle {
	fn from(object: Artifact) -> Self {
		match object {
			Artifact::Directory(directory) => directory.handle().clone(),
			Artifact::File(file) => file.handle().clone(),
			Artifact::Symlink(symlink) => symlink.handle().clone(),
		}
	}
}

impl From<Artifact> for Value {
	fn from(object: Artifact) -> Self {
		match object {
			Artifact::Directory(directory) => directory.into(),
			Artifact::File(file) => file.into(),
			Artifact::Symlink(symlink) => symlink.into(),
		}
	}
}

impl TryFrom<Value> for Artifact {
	type Error = Error;

	fn try_from(object: Value) -> Result<Self, Self::Error> {
		match object {
			Value::Directory(directory) => Ok(Self::Directory(directory)),
			Value::File(file) => Ok(Self::File(file)),
			Value::Symlink(symlink) => Ok(Self::Symlink(symlink)),
			_ => return_error!("Expected an artifact."),
		}
	}
}
