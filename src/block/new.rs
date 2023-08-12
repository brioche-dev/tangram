use super::Block;
use crate::{
	error::{return_error, Result},
	id::{self, Id},
};
use num_traits::ToPrimitive;
use std::collections::HashMap;
use varint_rs::VarintWriter;

impl Block {
	#[must_use]
	pub fn with_id(id: Id) -> Block {
		Block {
			id,
			bytes: None,
			children: None,
		}
	}

	pub fn empty() -> Result<Block> {
		Self::with_bytes(Box::new([]))
	}

	pub fn with_bytes(bytes: Box<[u8]>) -> Result<Self> {
		// Compute the block's ID.
		let id = Id::with_bytes(&bytes);

		// Create the block.
		let block = Self::new(id, bytes, HashMap::default());

		Ok(block)
	}

	pub fn with_data(data: &[u8]) -> Result<Self> {
		Self::with_children_and_data(vec![], data)
	}

	pub fn with_id_and_bytes(id: Id, bytes: Box<[u8]>) -> Result<Self> {
		// Verify the block's ID.
		if id != Id::with_bytes(&bytes) {
			return_error!("Invalid block ID.");
		}

		// Create the block.
		let block = Self::new(id, bytes, HashMap::default());

		Ok(block)
	}

	pub fn with_children_and_data(children: Vec<Block>, data: &[u8]) -> Result<Self> {
		// Create the block's bytes.
		let mut bytes = Vec::new();
		bytes.write_u64_varint(children.len().to_u64().unwrap())?;
		for child in &children {
			bytes.extend_from_slice(&child.id().as_bytes());
		}
		bytes.extend_from_slice(data);
		let bytes = bytes.into_boxed_slice();

		// Compute the block's ID.
		let id = Id::with_bytes(&bytes);

		// Collect the children.
		let children = children
			.into_iter()
			.map(|block| (block.id(), block))
			.collect();

		// Create the block.
		let block = Self::new(id, bytes, children);

		Ok(block)
	}

	fn new(id: Id, bytes: Box<[u8]>, children: HashMap<Id, Block, id::BuildHasher>) -> Self {
		Self {
			id,
			bytes: Some(bytes.into()),
			children: None,
		}
	}
}
