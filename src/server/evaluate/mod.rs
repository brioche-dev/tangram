use crate::{expression::Expression, server::Server, value::Value};
use anyhow::Result;
use async_recursion::async_recursion;
use futures::TryFutureExt;
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
	pub async fn evaluate(self: &Arc<Self>, expression: Expression) -> Result<Value> {
		// Recursively evaluate the expression.
		let value = match expression {
			Expression::Null => Value::Null,
			Expression::Bool(value) => Value::Bool(value),
			Expression::Number(value) => Value::Number(value),
			Expression::String(value) => Value::String(value),
			Expression::Artifact(artifact) => Value::Artifact(artifact),
			Expression::Path(path) => self.evaluate_path(path).await?,
			Expression::Template(template) => self.evaluate_template(template).await?,
			Expression::Fetch(fetch) => {
				// Return a memoized value if one is available.
				let fetch_expression = Expression::Fetch(fetch);
				if let Some(value) = self
					.get_memoized_value_for_expression(&fetch_expression)
					.await?
				{
					return Ok(value);
				}

				let fetch = match fetch_expression {
					Expression::Fetch(fetch) => fetch,
					_ => unreachable!(),
				};
				// Evaluate.
				let value = self.evaluate_fetch(&fetch).await?;

				// Memoize the value.
				self.set_memoized_value_for_expression(&Expression::Fetch(fetch), &value)
					.await?;

				value
			},
			Expression::Process(process) => self.evaluate_process(process).await?,
			Expression::Target(target) => self.evaluate_target(target).await?,
			Expression::Array(value) => {
				let values = value.into_iter().map(|value| self.evaluate(value));
				let array = futures::future::try_join_all(values).await?;
				Value::Array(array)
			},
			Expression::Map(value) => {
				let values = value.into_iter().map(|(key, expression)| {
					self.evaluate(expression).map_ok(|value| (key, value))
				});
				let value = futures::future::try_join_all(values).await?;
				let map = value.into_iter().collect();
				Value::Map(map)
			},
		};

		Ok(value)
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
		let value = self.evaluate(expression).await?;

		// Create the response.
		let body = serde_json::to_vec(&value)?;
		let response = http::Response::builder()
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}
}
