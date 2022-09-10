use crate::{expression::Expression, server::Server};
use anyhow::Result;
use async_recursion::async_recursion;
use futures::{future::try_join_all, TryFutureExt};
use std::sync::Arc;

mod expression;
mod fetch;
mod path;
mod process;
mod target;
mod template;

impl Server {
	/// Evaluate an [`Expression`].
	#[async_recursion]
	#[must_use]
	pub async fn evaluate(self: &Arc<Self>, expression: &Expression) -> Result<Expression> {
		match expression {
			Expression::Null => Ok(Expression::Null),
			Expression::Bool(value) => Ok(Expression::Bool(*value)),
			Expression::Number(value) => Ok(Expression::Number(*value)),
			Expression::String(value) => Ok(Expression::String(Arc::clone(value))),
			Expression::Artifact(artifact) => Ok(Expression::Artifact(*artifact)),
			Expression::Path(path) => {
				let output = self.evaluate_path(path).await?;
				Ok(output)
			},
			Expression::Template(template) => {
				let output = self.evaluate_template(template).await?;
				Ok(output)
			},
			Expression::Fetch(fetch) => {
				let output = self.evaluate_fetch(fetch).await?;
				Ok(output)
			},
			Expression::Process(process) => {
				let output = self.evaluate_process(process).await?;
				Ok(output)
			},
			Expression::Target(target) => {
				let output = self.evaluate_target(target).await?;
				Ok(output)
			},
			Expression::Array(array) => {
				let outputs = array.iter().map(|expression| self.evaluate(expression));
				let outputs = try_join_all(outputs).await?;
				let output = Expression::Array(outputs);
				Ok(output)
			},
			Expression::Map(map) => {
				let outputs = map.iter().map(|(key, expression)| {
					self.evaluate(expression)
						.map_ok(|value| (key.clone(), value))
				});
				let outputs = try_join_all(outputs).await?.into_iter().collect();
				let output = Expression::Map(outputs);
				Ok(output)
			},
		}
	}
}

impl Server {
	pub(super) async fn handle_evaluate_expression_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the request.
		let body = hyper::body::to_bytes(request).await?;
		let expression = serde_json::from_slice(&body)?;

		// Evaluate the expression.
		let output = self.evaluate(&expression).await?;

		// Create the response.
		let body = serde_json::to_vec(&output)?;
		let response = http::Response::builder()
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}
}
