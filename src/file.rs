use crate::{artifact, blob, error::Result, Instance};
use tokio::io::AsyncReadExt;

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct File {
	#[buffalo(id = 0)]
	#[serde(rename = "blobHash")]
	blob_hash: blob::Hash,

	#[buffalo(id = 1)]
	executable: bool,

	#[buffalo(id = 2)]
	references: Vec<artifact::Hash>,
}

impl File {
	#[must_use]
	pub fn new(blob_hash: blob::Hash, executable: bool, references: Vec<artifact::Hash>) -> Self {
		Self {
			blob_hash,
			executable,
			references,
		}
	}

	#[must_use]
	pub fn blob_hash(&self) -> blob::Hash {
		self.blob_hash
	}

	#[must_use]
	pub fn executable(&self) -> bool {
		self.executable
	}

	#[must_use]
	pub fn references(&self) -> &[artifact::Hash] {
		&self.references
	}

	pub async fn read_to_string(&self, tg: &Instance) -> Result<String> {
		let mut blob = tg.get_blob(self.blob_hash).await?;
		let mut string = String::new();
		blob.read_to_string(&mut string).await?;
		Ok(string)
	}
}
