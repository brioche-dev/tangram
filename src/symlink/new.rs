use super::Symlink;
use crate::{
	artifact::{self, Artifact},
	block::Block,
	error::Result,
	template::Template,
};
use itertools::Itertools;

impl Symlink {
	pub fn new(target: Template) -> Result<Self> {
		// Collect the children.
		let children = target
			.artifacts()
			.map(Artifact::block)
			.cloned()
			.collect_vec();

		// Create the artifact data.
		let data = artifact::Data::Symlink(super::Data {
			target: target.to_data(),
		});

		// Serialize the artifact data.
		let mut bytes = Vec::new();
		data.serialize(&mut bytes).unwrap();
		let data = bytes;

		// Create the block.
		let block = Block::with_children_and_data(children, &data)?;

		// Create the symlink.
		let symlink = Self { block, target };

		Ok(symlink)
	}
}
