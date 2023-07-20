pub use self::{data::Data, reader::Reader};
use crate::{
	block::Block,
	error::{Error, Result},
	instance::Instance,
};
use tokio::io::AsyncReadExt;

mod data;
mod get;
mod new;
mod reader;

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Blob {
	block: Block,
	kind: Kind,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
enum Kind {
	Branch(Vec<(Block, u64)>),
	Leaf(u64),
}

impl Blob {
	#[must_use]
	pub fn block(&self) -> Block {
		self.block
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
