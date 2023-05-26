pub use self::hash::Hash;
use crate::{
	error::{Error, Result},
	instance::Instance,
};
use std::path::PathBuf;

mod copy;
mod hash;
mod new;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Blob {
	hash: Hash,
}

impl Blob {
	#[must_use]
	pub(crate) fn with_hash(hash: Hash) -> Self {
		Self { hash }
	}

	#[must_use]
	pub fn hash(&self) -> Hash {
		self.hash
	}

	pub fn path(&self, tg: &Instance) -> PathBuf {
		tg.blob_path(self.hash)
	}

	pub async fn bytes(&self, tg: &Instance) -> Result<Vec<u8>> {
		let path = tg.blob_path(self.hash);
		let bytes = tokio::fs::read(&path).await?;
		Ok(bytes)
	}

	pub async fn text(&self, tg: &Instance) -> Result<String> {
		let bytes = self.bytes(tg).await?;
		let string = String::from_utf8(bytes).map_err(Error::other)?;
		Ok(string)
	}
}
