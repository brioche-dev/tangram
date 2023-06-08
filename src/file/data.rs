use super::File;
use crate::{
	artifact,
	blob::{self, Blob},
};

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
#[serde(rename_all = "camelCase")]
pub struct Data {
	#[buffalo(id = 0)]
	pub blob_hash: blob::Hash,

	#[buffalo(id = 1)]
	pub executable: bool,

	#[buffalo(id = 2)]
	pub references: Vec<artifact::Hash>,
}

impl File {
	#[must_use]
	pub fn to_data(&self) -> Data {
		Data {
			blob_hash: self.blob.hash(),
			executable: self.executable,
			references: self.references.clone(),
		}
	}

	#[must_use]
	pub fn from_data(hash: artifact::Hash, data: Data) -> Self {
		let blob = Blob::from_hash(data.blob_hash);
		let executable = data.executable;
		let references = data.references;
		Self {
			hash,
			blob,
			executable,
			references,
		}
	}
}
