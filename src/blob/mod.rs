pub use self::{data::Data, reader::Reader};
use crate::{
	block::Block,
	error::{Error, Result},
	id::Id,
	instance::Instance,
	target::{FromV8, ToV8},
};
use tokio::io::AsyncReadExt;

mod data;
mod get;
mod new;
mod reader;

#[derive(Clone, Debug)]
pub struct Blob {
	block: Block,
	kind: Kind,
}

#[derive(Clone, Debug)]
enum Kind {
	Branch(Vec<(Block, u64)>),
	Leaf(u64),
}

impl Blob {
	#[must_use]
	pub fn id(&self) -> Id {
		self.block().id()
	}

	#[must_use]
	pub fn block(&self) -> &Block {
		&self.block
	}

	pub async fn store(&self, tg: &Instance) -> Result<()> {
		self.block().store(tg).await
	}

	#[must_use]
	pub fn size(&self) -> u64 {
		match &self.kind {
			Kind::Branch(sizes) => sizes.iter().map(|(_, size)| size).sum(),
			Kind::Leaf(size) => *size,
		}
	}

	pub async fn bytes(&self, tg: &Instance) -> Result<Vec<u8>> {
		let mut reader = self.reader(tg);
		let mut bytes = Vec::new();
		reader.read_to_end(&mut bytes).await?;
		Ok(bytes)
	}

	pub async fn text(&self, tg: &Instance) -> Result<String> {
		let bytes = self.bytes(tg).await?;
		let string = String::from_utf8(bytes).map_err(Error::other)?;
		Ok(string)
	}
}

impl std::cmp::PartialEq for Blob {
	fn eq(&self, other: &Self) -> bool {
		self.id() == other.id()
	}
}

impl std::cmp::Eq for Blob {}

impl std::cmp::PartialOrd for Blob {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.id().partial_cmp(&other.id())
	}
}

impl std::cmp::Ord for Blob {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.id().cmp(&other.id())
	}
}

impl std::hash::Hash for Blob {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.id().hash(state);
	}
}

impl ToV8 for Blob {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		todo!()
	}
}

impl FromV8 for Blob {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		todo!()
	}
}
