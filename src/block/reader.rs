use super::Block;
use crate::{
	error::{Result, WrapErr},
	id::Id,
};
use num_traits::ToPrimitive;
use varint_rs::VarintReader;

pub struct Reader<R>
where
	R: std::io::Read + std::io::Seek,
{
	reader: R,
}

impl<R> Reader<R>
where
	R: std::io::Read + std::io::Seek,
{
	pub fn new(reader: R) -> Reader<R> {
		Reader { reader }
	}

	pub fn into_inner(self) -> R {
		self.reader
	}
}

impl<R> Reader<R>
where
	R: std::io::Read + std::io::Seek,
{
	pub fn size(&mut self) -> Result<u64> {
		let end = self.reader.seek(std::io::SeekFrom::End(0))?;
		let start = self.reader.seek(std::io::SeekFrom::Start(0))?;
		let size = end - start;
		Ok(size)
	}

	pub fn bytes(&mut self) -> Result<Vec<u8>> {
		self.reader.seek(std::io::SeekFrom::Start(0))?;
		let mut bytes = Vec::new();
		self.reader.read_to_end(&mut bytes)?;
		Ok(bytes)
	}

	pub fn children(&mut self) -> Result<Vec<Block>> {
		self.reader.seek(std::io::SeekFrom::Start(0))?;
		let children_count = self
			.reader
			.read_u64_varint()?
			.to_usize()
			.wrap_err("Invalid children count.")?;
		let mut children = Vec::with_capacity(children_count);
		for _ in 0..children_count {
			let mut id = [0; 32];
			self.reader.read_exact(&mut id)?;
			let id = Id::from(id);
			let block = Block::with_id(id);
			children.push(block);
		}
		Ok(children)
	}

	pub fn data_size(&mut self) -> Result<usize> {
		self.reader.seek(std::io::SeekFrom::Start(0))?;
		let children_count = self
			.reader
			.read_u64_varint()?
			.to_usize()
			.wrap_err("Invalid children count.")?;
		let start = self.reader.seek(std::io::SeekFrom::Current(
			(children_count * 32).to_i64().unwrap(),
		))?;
		let end = self.reader.seek(std::io::SeekFrom::End(0))?;
		let size = (end - start).to_usize().unwrap();
		Ok(size)
	}

	pub fn data(&mut self) -> Result<Vec<u8>> {
		self.reader.seek(std::io::SeekFrom::Start(0))?;
		let children_count = self.reader.read_u64_varint()?;
		self.reader.seek(std::io::SeekFrom::Current(
			(children_count * 32).to_i64().unwrap(),
		))?;
		let mut bytes = Vec::new();
		self.reader.read_to_end(&mut bytes)?;
		Ok(bytes)
	}
}
