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
	pub blob_hash: blob::Hash,

	#[buffalo(id = 1)]
	pub executable: bool,

	#[buffalo(id = 2)]
	pub references: Vec<artifact::Hash>,
}

impl File {
	#[must_use]
	pub fn new(blob_hash: blob::Hash) -> Self {
		Self {
			blob_hash,
			executable: false,
			references: Vec::new(),
		}
	}

	pub async fn read_to_string(&self, tg: &Instance) -> Result<String> {
		let mut blob = tg.get_blob(self.blob_hash).await?;
		let mut string = String::new();
		blob.read_to_string(&mut string).await?;
		Ok(string)
	}
}
