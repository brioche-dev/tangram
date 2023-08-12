use super::File;
use crate::{
	artifact::{self, Artifact},
	blob::Blob,
	block::Block,
	error::Result,
	instance::Instance,
};
use itertools::Itertools;

impl File {
	pub async fn new(
		tg: &Instance,
		contents: &Blob,
		executable: bool,
		references: &[Artifact],
	) -> Result<Self> {
		let references = references
			.iter()
			.map(Artifact::block)
			.cloned()
			.collect_vec();

		// Collect the children.
		let children = Some(contents.block().clone())
			.into_iter()
			.chain(references.iter().cloned())
			.collect_vec();

		// Create the artifact data.
		let data = artifact::Data::File(super::Data {
			contents: contents.id(),
			executable,
			references: references.iter().map(Block::id).collect(),
		});

		// Serialize the data.
		let mut bytes = Vec::new();
		data.serialize(&mut bytes).unwrap();
		let data = bytes;

		// Create the block.
		let block = Block::with_children_and_data(children, &data)?;

		// Create the file.
		let file = Self {
			block,
			contents: contents.block().clone(),
			executable,
			references,
		};

		Ok(file)
	}
}
