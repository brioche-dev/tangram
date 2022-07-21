use crate::artifact::ArtifactHash;
use std::collections::BTreeMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Lockfile {
	pub dependencies: BTreeMap<String, Entry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Entry {
	pub artifact_hash: ArtifactHash,
	pub dependencies: BTreeMap<String, Entry>,
}
