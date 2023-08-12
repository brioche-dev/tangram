use super::Directory;
use crate::{block::Block, id::Id};
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
	pub entries: BTreeMap<String, Id>,
}

impl Directory {
	#[must_use]
	pub fn to_data(&self) -> Data {
		let entries = self
			.entries
			.iter()
			.map(|(name, block)| (name.clone(), block.id()))
			.collect();
		Data { entries }
	}

	#[must_use]
	pub fn from_data(block: Block, data: Data) -> Self {
		let entries = data
			.entries
			.into_iter()
			.map(|(name, id)| (name, Block::with_id(id)))
			.collect();
		Self { block, entries }
	}
}
