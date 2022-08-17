use crate::object::ObjectHash;
use std::collections::BTreeMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Lockfile(pub BTreeMap<String, Entry>);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Entry {
	pub package: ObjectHash,
	pub dependencies: Lockfile,
}
