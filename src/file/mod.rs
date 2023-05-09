pub use self::data::Data;
use crate::{
	artifact::{self, Artifact},
	blob::Blob,
	error::{Error, Result},
	instance::Instance,
};
use futures::{stream::FuturesOrdered, TryStreamExt};

mod builder;
mod data;
mod new;

/// A file.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct File {
	/// The file's hash.
	hash: artifact::Hash,

	/// The file's blob.
	blob: Blob,

	/// Whether the file is executable.
	executable: bool,

	/// The file's references.
	references: Vec<artifact::Hash>,
}

impl File {
	#[must_use]
	pub fn hash(&self) -> artifact::Hash {
		self.hash
	}

	#[must_use]
	pub fn blob(&self) -> Blob {
		self.blob
	}

	#[must_use]
	pub fn executable(&self) -> bool {
		self.executable
	}

	pub async fn references(&self, tg: &Instance) -> Result<Vec<Artifact>> {
		let references = self
			.references
			.iter()
			.map(|hash| async move {
				let artifact = Artifact::get(tg, *hash).await?;
				Ok::<_, Error>(artifact)
			})
			.collect::<FuturesOrdered<_>>()
			.try_collect()
			.await?;
		Ok(references)
	}
}
