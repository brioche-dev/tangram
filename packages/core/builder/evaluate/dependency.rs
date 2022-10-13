use crate::{
	builder::Shared,
	expression::{self, Expression},
	hash::Hash,
};
use anyhow::Result;

impl Shared {
	pub(super) async fn evaluate_dependency(
		&self,
		hash: Hash,
		dependency: &expression::Dependency,
	) -> Result<Hash> {
		// Evaluate the artifact.
		let artifact = self.evaluate(dependency.artifact, hash).await?;

		// Create the output.
		let output = Expression::Dependency(expression::Dependency {
			artifact,
			path: dependency.path.clone(),
		});
		let output_hash = self.add_expression(&output).await?;

		Ok(output_hash)
	}
}
