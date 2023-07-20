use super::Symlink;
use crate::{
	artifact::{self, Artifact},
	block::Block,
	error::Result,
	instance::Instance,
	template::Template,
};
use itertools::Itertools;

impl Symlink {
	pub fn new(tg: &Instance, target: Template) -> Result<Self> {
		// Create the artifact data.
		let data = artifact::Data::Symlink(super::Data {
			target: target.to_data(),
		});

		// Serialize the artifact data.
		let mut bytes = Vec::new();
		data.serialize(&mut bytes).unwrap();

		// Collect the children.
		let children = target.artifacts().map(Artifact::block).collect_vec();

		// Create the block.
		let block = Block::new(tg, children, &bytes)?;

		// Create the symlink.
		let symlink = Self { block, target };

		Ok(symlink)
	}
}
