use super::Dependency;
use crate::id::Id;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Lockfile {
	pub dependencies: BTreeMap<Dependency, Entry>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Entry {
	Locked(Id),
	Unlocked {
		id: Id,
		dependencies: BTreeMap<Dependency, Entry>,
	},
}
