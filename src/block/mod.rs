pub use self::reader::Reader;
use crate::target::{FromV8, ToV8};
use crate::{
	bytes::Bytes,
	error::{Result, WrapErr},
	id::{self, Id},
	instance::Instance,
};
use lmdb::Transaction;
use std::{collections::HashMap, sync::Arc};

mod get;
mod new;
mod reader;
mod store;

#[derive(Clone, Debug)]
pub struct Block {
	id: Id,
	bytes: Option<Bytes>,
	children: Option<Arc<HashMap<Id, Block, id::BuildHasher>>>,
}

impl Block {
	#[must_use]
	pub fn id(&self) -> Id {
		self.id
	}

	pub async fn reader<'a>(&'a self, tg: &'a Instance) -> Result<Reader<'a>> {
		self.try_get_reader(tg)
			.await?
			.wrap_err("Failed to get the block reader.")
	}

	pub async fn try_get_reader<'a>(&'a self, tg: &'a Instance) -> Result<Option<Reader<'a>>> {
		// If the block is loaded, then return the reader.
		if let Some(reader) = self.try_get_reader_loaded().await? {
			return Ok(Some(reader));
		}

		// If the block is stored, then return the reader.
		if let Some(reader) = self.try_get_reader_stored(tg).await? {
			return Ok(Some(reader));
		}

		// Otherwise, return `None`.
		Ok(None)
	}

	#[allow(clippy::unused_async)]
	pub async fn try_get_reader_loaded(&self) -> Result<Option<Reader>> {
		let Some(bytes) = self.bytes.clone() else {
			return Ok(None);
		};
		let reader = Reader::new_loaded(bytes);
		Ok(Some(reader))
	}

	#[allow(clippy::unused_async)]
	pub async fn try_get_reader_stored<'a>(&self, tg: &'a Instance) -> Result<Option<Reader<'a>>> {
		// Start a transaction.
		let txn = tg.store.env.begin_ro_txn()?;

		// Create the reader.
		let stored = match reader::Stored::try_new(txn, |txn| {
			txn.get(tg.store.blocks, &self.id().as_bytes())
		}) {
			Ok(bytes) => bytes,
			Err(lmdb::Error::NotFound) => return Ok(None),
			Err(error) => return Err(error.into()),
		};
		let reader = Reader::new_stored(stored);

		Ok(Some(reader))
	}
}

impl From<Block> for Id {
	fn from(value: Block) -> Self {
		value.id
	}
}

impl From<Id> for Block {
	fn from(value: Id) -> Self {
		Self {
			id: value,
			bytes: None,
			children: None,
		}
	}
}

impl std::cmp::PartialEq for Block {
	fn eq(&self, other: &Self) -> bool {
		self.id() == other.id()
	}
}

impl std::cmp::Eq for Block {}

impl std::cmp::PartialOrd for Block {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.id().partial_cmp(&other.id())
	}
}

impl std::cmp::Ord for Block {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.id().cmp(&other.id())
	}
}

impl std::hash::Hash for Block {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.id().hash(state);
	}
}

impl ToV8 for Block {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		todo!()
	}
}

impl FromV8 for Block {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		todo!()
	}
}
