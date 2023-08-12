use crate::{
	bytes::Bytes,
	error::{Result, WrapErr},
	id::Id,
};
use num_traits::ToPrimitive;
use ouroboros::self_referencing;
use std::io::Read;
use varint_rs::VarintReader;

pub struct Reader<'a> {
	inner: Inner<'a>,
}

enum Inner<'a> {
	Borrowed(&'a [u8]),
	Loaded(Bytes),
	Stored(Stored<'a>),
}

#[self_referencing(pub_extras)]
pub(crate) struct Stored<'a> {
	txn: lmdb::RoTransaction<'a>,
	#[borrows(txn)]
	bytes: &'this [u8],
}

impl<'a> Reader<'a> {
	pub fn new_borrowed(bytes: &'a [u8]) -> Self {
		Self {
			inner: Inner::Borrowed(bytes),
		}
	}

	pub fn new_loaded(bytes: Bytes) -> Self {
		Self {
			inner: Inner::Loaded(bytes),
		}
	}

	pub(crate) fn new_stored(stored: Stored<'a>) -> Self {
		Self {
			inner: Inner::Stored(stored),
		}
	}

	#[must_use]
	pub fn size(&self) -> u64 {
		self.bytes().len().to_u64().unwrap()
	}

	#[must_use]
	pub fn bytes(&'a self) -> &'a [u8] {
		match &self.inner {
			Inner::Borrowed(bytes) => bytes,
			Inner::Loaded(bytes) => bytes.as_ref(),
			Inner::Stored(bytes) => bytes.borrow_bytes(),
		}
	}

	pub fn children(&self) -> Result<Vec<Id>> {
		let mut bytes = self.bytes();
		let children_count = bytes
			.read_u64_varint()?
			.to_usize()
			.wrap_err("Invalid children count.")?;
		let mut children = Vec::with_capacity(children_count);
		for _ in 0..children_count {
			let mut id = [0; 32];
			bytes.read_exact(&mut id)?;
			let id = Id::from(id);
			children.push(id);
		}
		Ok(children)
	}

	pub fn data(&'a self) -> Result<&'a [u8]> {
		let mut bytes = self.bytes();
		let children_count = bytes
			.read_u64_varint()?
			.to_usize()
			.wrap_err("Invalid children count.")?;
		Ok(&bytes[children_count.to_usize().unwrap() * 32..])
	}
}
