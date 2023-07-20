pub use self::data::Data;
use crate::{
	artifact::Artifact,
	blob::{self, Blob},
	block::Block,
	error::{Error, Result},
	instance::Instance,
};
use futures::{stream::FuturesOrdered, TryStreamExt};

mod builder;
mod data;
mod new;

/// A file.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct File {
	/// The file's block.
	block: Block,

	/// The file's contents.
	contents: Block,

	/// Whether the file is executable.
	executable: bool,

	/// The file's references.
	references: Vec<Block>,
}

impl File {
	#[must_use]
	pub fn block(&self) -> Block {
		self.block
	}

	pub async fn contents(&self, tg: &Instance) -> Result<Blob> {
		Blob::get(tg, self.contents).await
	}

	#[must_use]
	pub fn executable(&self) -> bool {
		self.executable
	}

	pub async fn references(&self, tg: &Instance) -> Result<Vec<Artifact>> {
		let references = self
			.references
			.iter()
			.map(|block| async move {
				let artifact = Artifact::get(tg, *block).await?;
				Ok::<_, Error>(artifact)
			})
			.collect::<FuturesOrdered<_>>()
			.try_collect()
			.await?;
		Ok(references)
	}

	pub async fn reader(&self, tg: &Instance) -> Result<blob::Reader> {
		Ok(self.contents(tg).await?.reader(tg))
	}

	pub async fn size(&self, tg: &Instance) -> Result<u64> {
		Ok(self.contents(tg).await?.size())
	}
}
