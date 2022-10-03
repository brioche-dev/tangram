use crate::{builder, hash::Hash};
use anyhow::{anyhow, Result};
use async_recursion::async_recursion;

impl builder::Shared {
	/// Evaluate an [`Expression`].
	#[async_recursion]
	#[must_use]
	pub async fn evaluate(&self, hash: Hash, parent_hash: Hash) -> Result<Hash> {
		// Add the evaluation.
		self.add_evaluation(parent_hash, hash).await?;

		// Get the expression and the output hash if the expression was previously evaluated.
		let (expression, output_hash) = self.get_expression_with_output(hash).await?;

		// If the expression was previously evaluated, return the output hash.
		if let Some(output_hash) = output_hash {
			// Return the output hash.
			return Ok(output_hash);
		}

		// Try each evaluator until one is found that can evaluate the expression.
		let mut output_hash = None;
		for evaluator in &self.evaluators {
			output_hash = evaluator.evaluate(self, hash, &expression).await?;
			if output_hash.is_some() {
				break;
			}
		}

		// If none of the evaluators can evaluate the expression, return an error.
		let output_hash = output_hash.ok_or_else(|| {
			anyhow!(r#"There was no evaluator for the expression with hash "{hash}"."#)
		})?;

		// Set the expression output.
		self.set_expression_output(hash, output_hash).await?;

		Ok(output_hash)
	}
}
