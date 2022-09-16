use super::error::bad_request;
use crate::{hash::Hash, server::Server};
use anyhow::{anyhow, bail, Context, Result};
use async_recursion::async_recursion;
use std::sync::Arc;

impl Server {
	/// Evaluate an [`Expression`].
	#[async_recursion]
	#[must_use]
	pub async fn evaluate(self: &Arc<Self>, hash: Hash, parent_hash: Hash) -> Result<Hash> {
		let _guard = self.lock.lock_shared().await?;

		// Get the expression and the output hash if the expression was previously evaluated.
		let (expression, output_hash) = self.get_expression_with_output(hash).await?;

		// If the expression was previously evaluated, return the output hash.
		if let Some(output_hash) = output_hash {
			// Add the evaluation.
			self.add_evaluation(parent_hash, hash).await?;

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
			anyhow!(
				r#"There was no evaluator for the expression with hash "{}"."#,
				hash
			)
		})?;

		// Set the expression output.
		self.set_expression_output(hash, output_hash).await?;

		// Add the evaluation.
		self.add_evaluation(parent_hash, hash).await?;

		Ok(output_hash)
	}
}

impl Server {
	pub(super) async fn handle_evaluate_expression_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let hash = if let ["expressions", hash, "evaluate"] = path_components.as_slice() {
			hash
		} else {
			bail!("Unexpected path.")
		};
		let hash: Hash = match hash.parse() {
			Ok(hash) => hash,
			Err(_) => return Ok(bad_request()),
		};

		// Evaluate the expression.
		let output = self
			.evaluate(hash, hash)
			.await
			.context("Failed to evaluate the expression.")?;

		// Create the response.
		let body = serde_json::to_vec(&output)?;
		let response = http::Response::builder()
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}
}
