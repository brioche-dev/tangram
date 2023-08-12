use super::File;
use crate::{block::Block, id::Id};

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
	pub contents: Id,

	#[tangram_serialize(id = 1)]
	pub executable: bool,

	#[tangram_serialize(id = 2)]
	pub references: Vec<Id>,
}

impl File {
	#[must_use]
	pub fn to_data(&self) -> Data {
		Data {
			contents: self.contents.id(),
			executable: self.executable,
			references: self.references.iter().map(Block::id).collect(),
		}
	}

	#[must_use]
	pub fn from_data(block: Block, data: Data) -> Self {
		let contents = Block::with_id(data.contents);
		let executable = data.executable;
		let references = data.references.into_iter().map(Block::with_id).collect();
		Self {
			block,
			contents,
			executable,
			references,
		}
	}
}
