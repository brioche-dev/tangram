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
		let references = references.iter().map(Artifact::block).collect_vec();

		// Create the artifact data.
		let data = artifact::Data::File(super::Data {
			contents: contents.block(),
			executable,
			references: references.clone(),
		});

		// Serialize the data.
		let mut bytes = Vec::new();
		data.serialize(&mut bytes).unwrap();
		let data = bytes;

		// Collect the children.
		let children = Some(contents.block())
			.into_iter()
			.chain(references.iter().copied())
			.collect_vec();

		// Create the block.
		let block = Block::new(tg, children, &data).await?;

		// Create the file.
		let file = Self {
			block,
			contents: contents.block(),
			executable,
			references,
		};

		Ok(file)
	}
}
