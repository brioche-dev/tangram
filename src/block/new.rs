use super::Block;
use crate::{
	error::Result,
	id::{self, Id},
	instance::Instance,
};
use num_traits::ToPrimitive;
use varint_rs::VarintWriter;

impl Block {
	pub fn new(tg: &Instance, children: Vec<Block>, data: &[u8]) -> Result<Self> {
		// Create the block's bytes.
		let mut bytes = Vec::new();
		bytes.write_u64_varint(children.len().to_u64().unwrap())?;
		for child in children {
			bytes.extend_from_slice(child.id().as_slice());
		}
		bytes.extend_from_slice(data);

		// Create the block.
		let block = Self::with_bytes(tg, bytes)?;

		Ok(block)
	}

	pub fn with_bytes(tg: &Instance, bytes: impl AsRef<[u8]>) -> Result<Self> {
		let bytes = bytes.as_ref();

		// Create the block's ID.
		let mut writer = id::Writer::new();
		writer.update(bytes);
		let id = writer.finalize();

		// Create the block.
		let block = Self::add(tg, id, bytes)?;

		Ok(block)
	}

	pub(crate) fn add(tg: &Instance, id: Id, bytes: impl AsRef<[u8]>) -> Result<Self> {
		let bytes = bytes.as_ref();

		// Add the block to the database.
		let connection = tg.get_database_connection()?;
		let mut statement = connection.prepare_cached(
			"insert into blocks (id, bytes) values (?, ?) on conflict (id) do nothing",
		)?;
		statement.execute(rusqlite::params![id, bytes])?;

		// Create the block.
		let block = Block { id };

		Ok(block)
	}
}
