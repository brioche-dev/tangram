use super::Template;
use crate::{artifact::Artifact, error::Result, instance::Instance};
use std::collections::HashSet;

impl Template {
	/// Collect a template's references.
	#[must_use]
	pub fn references(&self) -> Vec<Artifact> {
		self.components
			.iter()
			.filter_map(|component| component.as_artifact().cloned())
			.collect()
	}

	/// Collect a template's recursive references.
	pub async fn collect_recursive_references(
		&self,
		tg: &Instance,
		references: &mut HashSet<Artifact, fnv::FnvBuildHasher>,
	) -> Result<()> {
		for artifact in self.references() {
			references.insert(artifact.clone());
			artifact
				.collect_recursive_references(tg, references)
				.await?;
		}
		Ok(())
	}
}
