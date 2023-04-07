use super::Directory;
use crate::artifact;
use std::collections::BTreeMap;

#[derive(
	Clone,
	Debug,
	Default,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Data {
	#[buffalo(id = 0)]
	pub entries: BTreeMap<String, artifact::Hash>,
}

impl Directory {
	#[must_use]
	pub fn to_data(&self) -> Data {
		Data {
			entries: self.entries.clone(),
		}
	}

	#[must_use]
	pub fn from_data(hash: artifact::Hash, data: Data) -> Self {
		let entries = data.entries;
		Self { hash, entries }
	}
}
