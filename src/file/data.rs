use super::File;
use crate::block::Block;

#[derive(
	Clone,
	Debug,
	Eq,
	PartialEq,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(rename_all = "camelCase")]
pub struct Data {
	#[tangram_serialize(id = 0)]
	pub contents: Block,

	#[tangram_serialize(id = 1)]
	pub executable: bool,

	#[tangram_serialize(id = 2)]
	pub references: Vec<Block>,
}

impl File {
	#[must_use]
	pub fn to_data(&self) -> Data {
		Data {
			contents: self.contents,
			executable: self.executable,
			references: self.references.clone(),
		}
	}

	#[must_use]
	pub fn from_data(block: Block, data: Data) -> Self {
		let contents = data.contents;
		let executable = data.executable;
		let references = data.references;
		Self {
			block,
			contents,
			executable,
			references,
		}
	}
}
