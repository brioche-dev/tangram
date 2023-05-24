use super::{Dependency, Hash};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Lockfile {
	pub dependencies: BTreeMap<Dependency, Entry>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Entry {
	Locked(Hash),
	Unlocked {
		hash: Hash,
		dependencies: BTreeMap<Dependency, Entry>,
	},
}
