use crate::object::ObjectHash;
use std::collections::BTreeMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "version")]
pub enum Lockfile {
	#[serde(rename = "1")]
	V1(V1),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct V1 {
	pub dependencies: BTreeMap<String, Dependency>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Dependency {
	pub hash: ObjectHash,
	pub dependencies: Option<BTreeMap<String, Dependency>>,
}

impl Lockfile {
	#[must_use]
	pub fn new_v1(dependencies: BTreeMap<String, Dependency>) -> Lockfile {
		Lockfile::V1(V1 { dependencies })
	}

	#[must_use]
	pub fn as_v1(&self) -> Option<&V1> {
		match self {
			Lockfile::V1(v1) => Some(v1),
		}
	}
}
