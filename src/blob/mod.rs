pub use self::hash::Hash;
use self::reader::Reader;
use crate::{
	error::{Error, Result, WrapErr},
	instance::Instance,
};
use std::path::PathBuf;
use tokio::io::AsyncReadExt;

mod copy;
mod hash;
mod new;
mod reader;

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

	pub async fn reader(&self, tg: &Instance) -> Result<Option<Reader>> {
		// Get the path.
		let path = tg.blob_path(self.hash);

		// Open the file.
		let file = match tokio::fs::File::open(path).await {
			Ok(file) => file,
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
			Err(error) => return Err(error.into()),
		};

		// Create the reader.
		let reader = Reader { file };

		Ok(Some(reader))
	}

	pub async fn bytes(&self, tg: &Instance) -> Result<Vec<u8>> {
		let mut bytes = Vec::new();
		let mut reader = self.reader(tg).await?.wrap_err("The blob was not found.")?;
		reader.read_to_end(&mut bytes).await?;
		Ok(bytes)
	}

	pub async fn text(&self, tg: &Instance) -> Result<String> {
		let bytes = self.bytes(tg).await?;
		let string = String::from_utf8(bytes).map_err(Error::other)?;
		Ok(string)
	}
}
