use super::Directory;
use crate::block::Block;
use std::collections::BTreeMap;

#[derive(
	Clone,
	Debug,
	Default,
	Eq,
	PartialEq,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
pub struct Data {
	#[tangram_serialize(id = 0)]
	pub entries: BTreeMap<String, Block>,
}

impl Directory {
	#[must_use]
	pub fn to_data(&self) -> Data {
		Data {
			entries: self.entries.clone(),
		}
	}

	#[must_use]
	pub fn from_data(block: Block, data: Data) -> Self {
		let entries = data.entries;
		Self { block, entries }
	}
}
