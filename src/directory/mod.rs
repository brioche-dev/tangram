pub use self::{builder::Builder, data::Data};
use crate::{
	artifact::Artifact,
	block::Block,
	error::{Error, Result},
	instance::Instance,
};
use futures::{stream::FuturesOrdered, TryStreamExt};
use std::collections::BTreeMap;

mod builder;
mod data;
mod get;
mod new;

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Directory {
	block: Block,
	entries: BTreeMap<String, Block>,
}

impl Directory {
	/// Get the block.
	#[must_use]
	pub fn block(&self) -> Block {
		self.block
	}

	/// Get the entries.
	pub async fn entries(&self, tg: &Instance) -> Result<BTreeMap<String, Artifact>> {
		let entries = self
			.entries
			.iter()
			.map(|(name, hash)| async move {
				let artifact = Artifact::get(tg, *hash).await?;
				Ok::<_, Error>((name.clone(), artifact))
			})
			.collect::<FuturesOrdered<_>>()
			.try_collect()
			.await?;
		Ok(entries)
	}

	pub async fn try_get_entry(&self, tg: &Instance, name: &str) -> Result<Option<Artifact>> {
		let Some(block) = self.entries.get(name) else {
			return Ok(None);
		};
		let artifact = Artifact::get(tg, *block).await?;
		Ok(Some(artifact))
	}

	#[must_use]
	pub fn contains(&self, name: &str) -> bool {
		self.entries.contains_key(name)
	}

	pub fn names(&self) -> impl Iterator<Item = &str> {
		self.entries.keys().map(std::string::String::as_str)
	}

	pub async fn references(&self, tg: &Instance) -> Result<Vec<Artifact>> {
		Ok(self
			.entries(tg)
			.await?
			.into_values()
			.map(|artifact| async move { artifact.references(tg).await })
			.collect::<FuturesOrdered<_>>()
			.try_collect::<Vec<_>>()
			.await?
			.into_iter()
			.flatten()
			.collect())
	}
}
