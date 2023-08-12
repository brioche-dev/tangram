use super::Block;
use crate::{
	error::{Result, WrapErr},
	instance::Instance,
};
use num_traits::ToPrimitive;

impl Block {
	pub async fn is_local(&self, tg: &Instance) -> Result<bool> {
		Ok(self.try_get_reader_stored(tg).await?.is_some())
	}

	pub async fn size(&self, tg: &Instance) -> Result<u64> {
		self.try_get_size(tg)
			.await?
			.wrap_err("Failed to get the block.")
	}

	pub async fn try_get_size(&self, tg: &Instance) -> Result<Option<u64>> {
		let Some(reader) = self.try_get_reader(tg).await? else {
			return Ok(None);
		};
		let bytes = reader.bytes();
		Ok(Some(bytes.len().to_u64().unwrap()))
	}

	pub async fn bytes(&self, tg: &Instance) -> Result<Box<[u8]>> {
		self.try_get_bytes(tg)
			.await?
			.wrap_err("Failed to get the block.")
	}

	pub async fn try_get_bytes(&self, tg: &Instance) -> Result<Option<Box<[u8]>>> {
		let Some(reader) = self.try_get_reader(tg).await? else {
			return Ok(None);
		};
		let bytes = reader.bytes();
		Ok(Some(bytes.to_owned().into_boxed_slice()))
	}

	pub async fn children(&self, tg: &Instance) -> Result<Vec<Block>> {
		self.try_get_children(tg)
			.await?
			.wrap_err("Failed to get the block.")
	}

	pub async fn try_get_children(&self, tg: &Instance) -> Result<Option<Vec<Block>>> {
		let Some(reader) = self.try_get_reader(tg).await? else {
			return Ok(None);
		};
		let children = reader
			.children()?
			.into_iter()
			.map(|id| {
				let children = self.children.as_ref();
				if let Some(child) = children.and_then(|children| children.get(&id)) {
					child.clone()
				} else {
					Block::with_id(id)
				}
			})
			.collect();
		Ok(Some(children))
	}

	pub async fn data_size(&self, tg: &Instance) -> Result<u64> {
		self.try_get_data_size(tg)
			.await?
			.wrap_err("Failed to get the block.")
	}

	pub async fn try_get_data_size(&self, tg: &Instance) -> Result<Option<u64>> {
		let Some(reader) = self.try_get_reader(tg).await? else {
			return Ok(None);
		};
		let data = reader.data()?;
		Ok(Some(data.len().to_u64().unwrap()))
	}

	pub async fn data(&self, tg: &Instance) -> Result<Box<[u8]>> {
		self.try_get_data(tg)
			.await?
			.wrap_err("Failed to get the block.")
	}

	pub async fn try_get_data(&self, tg: &Instance) -> Result<Option<Box<[u8]>>> {
		let Some(reader) = self.try_get_reader(tg).await? else {
			return Ok(None);
		};
		let data = reader.data()?;
		Ok(Some(data.to_owned().into_boxed_slice()))
	}
}
