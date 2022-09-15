use super::error::bad_request;
use crate::{expression::Expression, hash::Hash, server::Server};
use anyhow::{bail, Context, Result};
use async_recursion::async_recursion;
use futures::{future::try_join_all, TryFutureExt};
use std::sync::Arc;

mod fetch;
mod path;
mod process;
mod target;
mod template;

impl Server {
	/// Evaluate an [`Expression`].
	#[async_recursion]
	#[must_use]
	pub async fn evaluate(self: &Arc<Self>, hash: Hash, parent_hash: Hash) -> Result<Hash> {
		let _guard = self.lock.lock_shared().await?;

		// Get the expression and the output hash if the expression was previously evaluated.
		let (expression, output_hash) = self.get_expression_with_output(hash).await?;

		if let Some(output_hash) = output_hash {
			// Add the evaluation.
			self.add_evaluation(parent_hash, hash).await?;

			// Return the output hash.
			return Ok(output_hash);
		}

		let output_hash = match &expression {
			Expression::Null
			| Expression::Bool(_)
			| Expression::Number(_)
			| Expression::String(_)
			| Expression::Artifact(_)
			| Expression::Directory(_)
			| Expression::File(_)
			| Expression::Symlink(_)
			| Expression::Dependency(_) => hash,
			Expression::Path(path) => self.evaluate_path(path, parent_hash).await?,
			Expression::Template(template) => self.evaluate_template(template, parent_hash).await?,
			Expression::Fetch(fetch) => self.evaluate_fetch(fetch).await?,
			Expression::Process(process) => self.evaluate_process(process, parent_hash).await?,
			Expression::Target(target) => self.evaluate_target(target, parent_hash).await?,
			Expression::Array(array) => {
				let output_hashes =
					try_join_all(array.iter().map(|hash| self.evaluate(*hash, parent_hash)))
						.await?;
				self.add_expression(&Expression::Array(output_hashes))
					.await?
			},
			Expression::Map(map) => {
				let outputs = try_join_all(map.iter().map(|(key, hash)| {
					self.evaluate(*hash, parent_hash)
						.map_ok(|value| (Arc::clone(key), value))
				}))
				.await?
				.into_iter()
				.collect();
				let output = Expression::Map(outputs);
				self.add_expression(&output).await?
			},
		};

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
