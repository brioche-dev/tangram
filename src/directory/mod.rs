pub use self::{builder::Builder, data::Data};
use crate::{
	artifact::{self, Artifact},
	error::{Error, Result},
	instance::Instance,
};
use futures::{stream::FuturesOrdered, TryStreamExt};
use std::collections::BTreeMap;

mod builder;
mod data;
mod get;
mod new;

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub struct Directory {
	hash: artifact::Hash,
	entries: BTreeMap<String, artifact::Hash>,
}

impl Directory {
	/// Get the hash.
	#[must_use]
	pub fn hash(&self) -> artifact::Hash {
		self.hash
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

	#[must_use]
	pub fn contains(&self, name: &str) -> bool {
		self.entries.contains_key(name)
	}

	pub fn names(&self) -> impl Iterator<Item = &str> {
		self.entries.keys().map(std::string::String::as_str)
	}
}
