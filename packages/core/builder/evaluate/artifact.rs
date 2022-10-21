use crate::{
	builder::State,
	expression::{self, Expression},
	hash::Hash,
};
use anyhow::Result;

impl State {
	pub(super) async fn evaluate_artifact(
		&self,
		hash: Hash,
		artifact: &expression::Artifact,
	) -> Result<Hash> {
		// Evaluate the artifact.
		let root = self.evaluate(artifact.root, hash).await?;

		// Create the output.
		let output = Expression::Artifact(expression::Artifact { root });
		let output_hash = self.add_expression(&output).await?;

		Ok(output_hash)
	}
}
