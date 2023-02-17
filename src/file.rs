use crate::blob;

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
