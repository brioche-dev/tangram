pub use self::hash::Hash;
use crate::{
	error::{Error, Result},
	instance::Instance,
};
use tokio::io::AsyncReadExt;

mod copy;
mod get;
mod hash;
mod new;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Blob {
	hash: Hash,
}

impl Blob {
	#[must_use]
	pub fn from_hash(hash: Hash) -> Self {
		Self { hash }
	}

	#[must_use]
	pub fn hash(&self) -> Hash {
		self.hash
	}

	pub async fn bytes(&self, tg: &Instance) -> Result<Vec<u8>> {
		let mut reader = self.get(tg).await?;
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
