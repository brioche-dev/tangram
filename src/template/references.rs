use super::{Component, Template};
use crate::{artifact, error::Result, Instance};

impl Template {
	// Collect a template's references.
	pub fn collect_references_into(&self, references: &mut Vec<artifact::Hash>) {
		for component in &self.components {
			if let Component::Artifact(artifact_hash) = component {
				references.push(*artifact_hash);
			}
		}
	}

	// Collect a template's recursive references.
	pub fn collect_recursive_references_into(
		&self,
		tg: &Instance,
		references: &mut Vec<artifact::Hash>,
	) -> Result<()> {
		for component in &self.components {
			if let Component::Artifact(artifact_hash) = component {
				references.push(*artifact_hash);
				let artifact = tg.get_artifact_local(*artifact_hash)?;
				artifact.collect_recursive_references_into(tg, references)?;
			}
		}
		Ok(())
	}
}
