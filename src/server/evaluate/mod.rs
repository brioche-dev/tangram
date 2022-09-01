use super::error::bad_request;
use crate::{expression::Expression, hash::Hash, server::Server, value::Value};
use anyhow::{bail, Context, Result};
use async_recursion::async_recursion;
use futures::TryFutureExt;
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
			Expression::Fetch(fetch) => self.evaluate_fetch(fetch).await?,
			Expression::Process(process) => self.evaluate_process(process).await?,
			Expression::Target(target) => {
				let expression = self.evaluate_target(target).await?;
				self.evaluate(expression).await?
			},
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

	/// Retrieve the memoized value from a previous evaluation of an expression, if one exists.
	pub(super) async fn get_memoized_value_for_expression(
		self: &Arc<Self>,
		expression: &Expression,
	) -> Result<Option<Value>> {
		let expression_json = serde_json::to_vec(&expression)?;
		let expression_hash = Hash::new(&expression_json);
		let value = self
			.get_memoized_value_for_expression_hash(&expression_hash)
			.await?;
		Ok(value)
	}

	/// Retrieve the memoized value from a previous evaluation of an expression, if one exists, given an expression hash.
	pub(super) async fn get_memoized_value_for_expression_hash(
		self: &Arc<Self>,
		expression_hash: &Hash,
	) -> Result<Option<Value>> {
		let value = self
			.database_query_row(
				r#"
					select value
					from expressions
					where hash = $1
				"#,
				(expression_hash.to_string(),),
				|row| Ok(row.get::<_, Vec<u8>>(0)?),
			)
			.await?;
		let value = if let Some(value) = value {
			let value = serde_json::from_slice(&value)?;
			Some(value)
		} else {
			None
		};
		Ok(value)
	}

	/// Memoize the value from the evaluation of an expression.
	pub(super) async fn set_memoized_value_for_expression(
		self: &Arc<Self>,
		expression: &Expression,
		value: &Value,
	) -> Result<()> {
		let expression_json = serde_json::to_vec(&expression)?;
		let expression_hash = Hash::new(&expression_json);
		let value_json = serde_json::to_vec(&value)?;
		self.database_execute(
			r#"
				insert into expressions (
					hash, data, value
				) values (
					$1, $2, $3
				)
			"#,
			(expression_hash.to_string(), expression_json, value_json),
		)
		.await?;
		Ok(())
	}
}

impl Server {
	pub(super) async fn handle_get_expression_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let expression_hash = if let &["expressions", expression_hash] = path_components.as_slice()
		{
			expression_hash
		} else {
			bail!("Unexpected path.");
		};
		let expression_hash = match expression_hash.parse() {
			Ok(expression_hash) => expression_hash,
			Err(_) => return Ok(bad_request()),
		};

		let value = self
			.get_memoized_value_for_expression_hash(&expression_hash)
			.await?;

		// Create the response.
		let body = serde_json::to_vec(&value).context("Failed to serialize the response body.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
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
