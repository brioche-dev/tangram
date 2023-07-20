use super::{Block, Reader};
use crate::{
	error::{Result, WrapErr},
	instance::Instance,
};
use tokio::io::AsyncReadExt;

impl Block {
	/// Determine whether the block is in this instance's database.
	pub fn is_local(&self, tg: &Instance) -> Result<bool> {
		Ok(self.try_get_row_id(tg)?.is_some())
	}

	pub async fn size(&self, tg: &Instance) -> Result<u64> {
		self.try_get_size(tg)
			.await?
			.wrap_err("Failed to get the block.")
	}

	pub async fn try_get_size(&self, tg: &Instance) -> Result<Option<u64>> {
		let Some(row_id) = self.try_get_internal(tg).await? else {
			return Ok(None);
		};
		let connection = tg.get_database_connection()?;
		let blob = connection.blob_open(rusqlite::MAIN_DB, "blocks", "bytes", row_id, true)?;
		let mut reader = Reader::new(blob);
		let size = reader.size()?;
		Ok(Some(size))
	}

	pub async fn bytes(&self, tg: &Instance) -> Result<Vec<u8>> {
		self.try_get_bytes(tg)
			.await?
			.wrap_err("Failed to get the block.")
	}

	pub async fn try_get_bytes(&self, tg: &Instance) -> Result<Option<Vec<u8>>> {
		let Some(row_id) = self.try_get_internal(tg).await? else {
			return Ok(None);
		};
		let connection = tg.get_database_connection()?;
		let blob = connection.blob_open(rusqlite::MAIN_DB, "blocks", "bytes", row_id, true)?;
		let mut reader = Reader::new(blob);
		let bytes = reader.bytes()?;
		Ok(Some(bytes))
	}

	pub async fn children(&self, tg: &Instance) -> Result<Vec<Block>> {
		self.try_get_children(tg)
			.await?
			.wrap_err("Failed to get the block.")
	}

	pub async fn try_get_children(&self, tg: &Instance) -> Result<Option<Vec<Block>>> {
		let Some(row_id) = self.try_get_internal(tg).await? else {
			return Ok(None);
		};
		let connection = tg.get_database_connection()?;
		let blob = connection.blob_open(rusqlite::MAIN_DB, "blocks", "bytes", row_id, true)?;
		let mut reader = Reader::new(blob);
		let children = reader.children()?;
		Ok(Some(children))
	}

	pub async fn data_size(&self, tg: &Instance) -> Result<usize> {
		self.try_get_data_size(tg)
			.await?
			.wrap_err("Failed to get the block.")
	}

	pub async fn try_get_data_size(&self, tg: &Instance) -> Result<Option<usize>> {
		let Some(row_id) = self.try_get_internal(tg).await? else {
			return Ok(None);
		};
		let connection = tg.get_database_connection()?;
		let blob = connection.blob_open(rusqlite::MAIN_DB, "blocks", "bytes", row_id, true)?;
		let mut reader = Reader::new(blob);
		let size = reader.data_size()?;
		Ok(Some(size))
	}

	pub async fn data(&self, tg: &Instance) -> Result<Vec<u8>> {
		self.try_get_data(tg)
			.await?
			.wrap_err("Failed to get the block.")
	}

	pub async fn try_get_data(&self, tg: &Instance) -> Result<Option<Vec<u8>>> {
		let Some(row_id) = self.try_get_internal(tg).await? else {
			return Ok(None);
		};
		let connection = tg.get_database_connection()?;
		let blob = connection.blob_open(rusqlite::MAIN_DB, "blocks", "bytes", row_id, true)?;
		let mut reader = Reader::new(blob);
		let data = reader.data()?;
		Ok(Some(data))
	}

	/// Attempt to get the block from the API if necessary and return the block's row ID in the database.
	async fn try_get_internal(&self, tg: &Instance) -> Result<Option<i64>> {
		// Attempt to get the block's row ID if it is in the database.
		if let Some(row_id) = self.try_get_row_id(tg)? {
			return Ok(Some(row_id));
		};

		// Otherwise, attempt to get the block from the API, add it to the database, and return the inserted row ID.
		let Some(mut reader) = tg.api_client.try_get_block(self.id()).await? else {
			return Ok(None);
		};
		let mut bytes = Vec::new();
		reader.read_to_end(&mut bytes).await?;
		let connection = tg.get_database_connection()?;
		let mut statement = connection.prepare_cached(
			"insert into blocks (id, bytes) values (?, ?) on conflict (id) do nothing",
		)?;
		statement.execute(rusqlite::params![self.id(), bytes])?;
		let row_id = connection.last_insert_rowid();
		Ok(Some(row_id))
	}

	/// Attempt to get the block's row ID in the database.
	fn try_get_row_id(&self, tg: &Instance) -> Result<Option<i64>> {
		let connection = tg.get_database_connection()?;
		let mut statement = connection.prepare_cached("select rowid from blocks where id = ?")?;
		let mut rows = statement.query(rusqlite::params![self.id(),])?;
		let Some(row) = rows.next()? else {
			return Ok(None);
		};
		let row_id = row.get(0)?;
		Ok(Some(row_id))
	}
}
