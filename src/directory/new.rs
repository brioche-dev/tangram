use super::Directory;
use crate::{
	artifact::{self, Artifact},
	block::Block,
	error::Result,
};
use itertools::Itertools;
use std::collections::BTreeMap;

impl Directory {
	pub async fn new(entries: &BTreeMap<String, Artifact>) -> Result<Self> {
		// Get the entries' blocks.
		let entries: BTreeMap<String, Block> = entries
			.iter()
			.map(|(name, artifact)| (name.clone(), artifact.block().clone()))
			.collect();

		// Collect the children.
		let children = entries.values().cloned().collect_vec();

		// Create the artifact data.
		let data = artifact::Data::Directory(super::Data {
			entries: entries
				.iter()
				.map(|(name, block)| (name.clone(), block.id()))
				.collect(),
		});

		// Serialize the data.
		let mut bytes = Vec::new();
		data.serialize(&mut bytes).unwrap();
		let data = bytes;

		// Create the block.
		let block = Block::with_children_and_data(children, &data)?;

		// Create the directory.
		let directory = Self { block, entries };

		Ok(directory)
	}
}
