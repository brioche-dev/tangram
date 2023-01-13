use crate::blob::BlobHash;

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
	pub blob: BlobHash,

	#[buffalo(id = 1)]
	pub executable: bool,
}
