use crate::{
	blob, checksum, directory, file, id, object, return_error, symlink, Blob, Checksum, Client,
	Directory, Error, File, Result, Symlink, Value,
};
use bytes::Bytes;
use derive_more::{From, TryInto, TryUnwrap};
use futures::stream::{FuturesUnordered, TryStreamExt};
use std::collections::{HashSet, VecDeque};

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
	Debug,
	Eq,
	From,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	TryInto,
	serde::Deserialize,
	serde::Serialize,
)]
#[serde(into = "crate::Id", try_from = "crate::Id")]
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

#[derive(Clone, Debug, From, TryUnwrap)]
#[try_unwrap(ref)]
pub enum Data {
	/// A directory.
	Directory(directory::Data),

	/// A file.
	File(file::Data),

	/// A symlink.
	Symlink(symlink::Data),
}

impl Id {
	#[must_use]
	pub fn to_bytes(&self) -> Bytes {
		match self {
			Self::Directory(id) => id.to_bytes(),
			Self::File(id) => id.to_bytes(),
			Self::Symlink(id) => id.to_bytes(),
		}
	}
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
			Self::Directory(directory) => Ok(directory.id(client).await?.clone().into()),
			Self::File(file) => Ok(file.id(client).await?.clone().into()),
			Self::Symlink(symlink) => Ok(symlink.id(client).await?.clone().into()),
		}
	}

	pub async fn data(&self, client: &dyn Client) -> Result<Data> {
		match self {
			Self::Directory(directory) => Ok(directory.data(client).await?.into()),
			Self::File(file) => Ok(file.data(client).await?.into()),
			Self::Symlink(symlink) => Ok(symlink.data(client).await?.into()),
		}
	}
}

impl Artifact {
	#[allow(clippy::unused_async)]
	pub async fn archive(
		&self,
		_client: &dyn Client,
		_format: blob::ArchiveFormat,
	) -> Result<Blob> {
		unimplemented!()
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
	) -> Result<HashSet<Id, fnv::FnvBuildHasher>> {
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
					Self::Symlink(symlink) => symlink
						.artifact(client)
						.await?
						.clone()
						.into_iter()
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
						let inserted = references.insert(artifact.id(client).await?);

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

impl std::str::FromStr for Id {
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

impl From<Id> for object::Id {
	fn from(value: Id) -> Self {
		match value {
			Id::Directory(id) => id.into(),
			Id::File(id) => id.into(),
			Id::Symlink(id) => id.into(),
		}
	}
}

impl TryFrom<object::Id> for Id {
	type Error = Error;

	fn try_from(value: object::Id) -> Result<Self, Self::Error> {
		match value {
			object::Id::Directory(value) => Ok(value.into()),
			object::Id::File(value) => Ok(value.into()),
			object::Id::Symlink(value) => Ok(value.into()),
			_ => return_error!("Expected an artifact ID."),
		}
	}
}

impl std::fmt::Display for Artifact {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Directory(directory) => write!(f, "{directory}"),
			Self::File(file) => write!(f, "{file}"),
			Self::Symlink(symlink) => write!(f, "{symlink}"),
		}
	}
}

impl From<Artifact> for Value {
	fn from(value: Artifact) -> Self {
		match value {
			Artifact::Directory(directory) => directory.into(),
			Artifact::File(file) => file.into(),
			Artifact::Symlink(symlink) => symlink.into(),
		}
	}
}

impl TryFrom<Value> for Artifact {
	type Error = Error;

	fn try_from(value: Value) -> Result<Self, Self::Error> {
		match value {
			Value::Directory(directory) => Ok(Self::Directory(directory)),
			Value::File(file) => Ok(Self::File(file)),
			Value::Symlink(symlink) => Ok(Self::Symlink(symlink)),
			_ => return_error!("Expected an artifact."),
		}
	}
}
