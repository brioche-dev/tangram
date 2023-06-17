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
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
#[serde(rename_all = "camelCase")]
pub struct Data {
	#[tangram_serialize(id = 0)]
	pub blob_hash: blob::Hash,

	#[tangram_serialize(id = 1)]
	pub executable: bool,

	#[tangram_serialize(id = 2)]
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
