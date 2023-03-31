use super::{Component, Template};
use crate::{artifact, error::Result, hash, Instance};
use std::collections::HashSet;

impl Template {
	// Collect a template's references.
	pub fn collect_references_into(
		&self,
		references: &mut HashSet<artifact::Hash, hash::BuildHasher>,
	) {
		for component in &self.components {
			if let Component::Artifact(artifact_hash) = component {
				references.insert(*artifact_hash);
			}
		}
	}

	// Collect a template's recursive references.
	pub fn collect_recursive_references_into(
		&self,
		tg: &Instance,
		references: &mut HashSet<artifact::Hash, hash::BuildHasher>,
	) -> Result<()> {
		for component in &self.components {
			if let Component::Artifact(artifact_hash) = component {
				references.insert(*artifact_hash);
				let artifact = tg.get_artifact_local(*artifact_hash)?;
				artifact.collect_recursive_references_into(tg, references)?;
			}
		}
		Ok(())
	}
}
