use super::Directory;
use crate::{
	artifact::{self, Artifact},
	block::Block,
	error::Result,
	instance::Instance,
};
use itertools::Itertools;
use std::collections::BTreeMap;

impl Directory {
	pub fn new(tg: &Instance, entries: &BTreeMap<String, Artifact>) -> Result<Self> {
		// Get the entries' blocks.
		let entries: BTreeMap<String, Block> = entries
			.iter()
			.map(|(name, artifact)| (name.clone(), artifact.block()))
			.collect();

		// Create the artifact data.
		let data = artifact::Data::Directory(super::Data {
			entries: entries.clone(),
		});

		// Serialize the data.
		let mut bytes = Vec::new();
		data.serialize(&mut bytes).unwrap();
		let data = bytes;

		// Collect the children.
		let children = entries.values().copied().collect_vec();

		// Create the block.
		let block = Block::new(tg, children, &data)?;

		// Create the directory.
		let directory = Self { block, entries };

		Ok(directory)
	}
}
