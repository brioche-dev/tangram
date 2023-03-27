use super::{Component, Template};
use crate::{artifact, error::Result, Instance};

impl Template {
	// Collect all artifact hashes a template references recursively.
	pub fn collect_references(
		&self,
		tg: &Instance,
		references: &mut Vec<artifact::Hash>,
	) -> Result<()> {
		for component in &self.components {
			if let Component::Artifact(artifact_hash) = component {
				references.push(*artifact_hash);
				let artifact = tg.get_artifact_local(*artifact_hash)?;
				artifact.collect_references(tg, references)?;
			}
		}
		Ok(())
	}
}
