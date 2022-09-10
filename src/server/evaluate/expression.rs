use crate::{
	expression::Expression,
	hash::Hash,
	server::{
		error::{bad_request, not_found},
		Server,
	},
	value::Value,
};
use anyhow::{bail, Context, Result};
use async_recursion::async_recursion;
use std::sync::Arc;

impl Server {
	/// Retrieve the memoized value from a previous evaluation of an expression, if one exists.
	#[async_recursion]
	#[must_use]
	pub async fn get_memoized_value_for_expression(
		self: &Arc<Self>,
		expression: &Expression,
	) -> Result<Option<Value>> {
		let expression_json = serde_json::to_vec(&expression)?;
		let expression_hash = Hash::new(&expression_json);

		// Check if we have memoized the expression.
		let value = self
			.get_memoized_value_for_expression_hash(&expression_hash)
			.await?;

		if value.is_some() {
			return Ok(value);
		}

		// Otherwise, check if any of our peers have memoized the expression.
		for client in &self.peers {
			let result = client.get_memoized_value_for_expression(expression).await;
			match result {
				Ok(value) => {
					if value.is_some() {
						return Ok(value);
					}
				},
				Err(_) => continue,
			}
		}

		Ok(None)
	}

	/// Retrieve the memoized value from a previous evaluation of an expression, if one exists, given an expression hash.
	pub async fn get_memoized_value_for_expression_hash(
		self: &Arc<Self>,
		expression_hash: &Hash,
	) -> Result<Option<Value>> {
		let value = self
			.database_transaction(|txn| {
				let sql = r#"
					select
						value
					from
						expressions
					where
						hash = $1
				"#;
				let params = (expression_hash.to_string(),);
				let mut statement = txn
					.prepare_cached(sql)
					.context("Failed to prepare the query.")?;
				let expression: Option<Vec<u8>> = statement
					.query(params)?
					.and_then(|row| row.get::<_, Vec<u8>>(0))
					.next()
					.transpose()?;
				Ok(expression)
			})
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
	pub async fn set_memoized_value_for_expression(
		self: &Arc<Self>,
		expression: &Expression,
		value: &Value,
	) -> Result<()> {
		let expression_json = serde_json::to_vec(&expression)?;
		let expression_hash = Hash::new(&expression_json);
		let value_json = serde_json::to_vec(&value)?;
		self.database_transaction(|txn| {
			txn.execute(
				r#"
					replace into expressions (
						hash, data, value
					) values (
						$1, $2, $3
					)
				"#,
				(expression_hash.to_string(), expression_json, value_json),
			)?;
			Ok(())
		})
		.await?;
		// TODO replace into the subexpressions table.
		Ok(())
	}
}
impl Server {
	pub async fn handle_get_expression_request(
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

		// If the value is None, return a 404
		let value = match value {
			Some(value) => value,
			None => return Ok(not_found()),
		};

		// Create the response.
		let body = serde_json::to_vec(&value).context("Failed to serialize the response body.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}
}
