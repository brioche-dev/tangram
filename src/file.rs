use crate::{blob, error::Result, Instance};

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
}

impl File {
	pub fn read_to_string(&self, _tg: &Instance) -> Result<String> {
		todo!()
	}
}
