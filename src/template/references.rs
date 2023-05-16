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

	/// Collect a template's references.
	pub fn collect_references(&self, references: &mut HashSet<Artifact, fnv::FnvBuildHasher>) {
		for component in &self.components {
			if let Some(artifact) = component.as_artifact() {
				references.insert(artifact.clone());
			}
		}
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
