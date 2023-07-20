pub use self::reader::Reader;
use crate::{error::Result, id::Id};

mod get;
mod new;
pub mod reader;

#[derive(
	Clone,
	Copy,
	Debug,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[tangram_serialize(into = "Id", try_from = "Id")]
pub struct Block {
	id: Id,
}

impl Block {
	#[must_use]
	pub fn with_id(id: Id) -> Block {
		Block { id }
	}

	#[must_use]
	pub fn id(&self) -> Id {
		self.id
	}
}

impl std::fmt::Display for Block {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.id)
	}
}

impl std::str::FromStr for Block {
	type Err = hex::FromHexError;

	fn from_str(s: &str) -> Result<Block, hex::FromHexError> {
		let id = Id::from_str(s)?;
		Ok(Block { id })
	}
}

impl From<Block> for Id {
	fn from(value: Block) -> Self {
		value.id
	}
}

impl From<Id> for Block {
	fn from(value: Id) -> Self {
		Self { id: value }
	}
}

impl std::hash::Hash for Block {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.id.hash(state);
	}
}
