use crate::{
	expression::{self, Expression},
	hash,
	server::{
		error::{bad_request, not_found},
		Server,
	},
};
use anyhow::{bail, Context, Result};
use async_recursion::async_recursion;
use futures::{future::select_ok, FutureExt};
use std::sync::Arc;

impl Server {
	/// Retrieve the memoized output from a previous evaluation of an expression, if one exists, either on this server or one of its peer servers.
	#[async_recursion]
	#[must_use]
	pub async fn get_memoized_evaluation(
		self: &Arc<Self>,
		expression_hash: expression::Hash,
	) -> Result<Option<Expression>> {
		// Check if we have memoized a previous evaluation of the expression.
		if let Some(output) = self.get_local_memoized_evaluation(&expression_hash).await? {
			return Ok(Some(output));
		}

		// Otherwise, check if any of our peers have memoized a previous evaluation of the expression.
		let peer_futures = self
			.peers
			.iter()
			.map(|peer| peer.get_memoized_evaluation(expression_hash).boxed());
		if let Ok((Some(output), _)) = select_ok(peer_futures).await {
			return Ok(Some(output));
		}

		// Otherwise, there is no memoized evaluation of the expression.
		Ok(None)
	}

	/// Memoize the output from the evaluation of an expression.
	pub async fn set_memoized_evaluation(
		self: &Arc<Self>,
		expression: &Expression,
		output: &Expression,
	) -> Result<()> {
		let expression_json = serde_json::to_vec(&expression)?;
		let expression_hash = expression::Hash(hash::Hash::new(&expression_json));
		let output_json = serde_json::to_vec(&output)?;
		let output_hash = expression::Hash(hash::Hash::new(&output_json));
		self.database_transaction(|txn| {
			let sql = r#"
				replace into evaluations (
					expression_hash, expression, output_hash, output
				) values (
					$1, $2, $3, $4
				)
			"#;
			let params = (
				expression_hash.to_string(),
				expression_json,
				output_hash.to_string(),
				output_json,
			);
			txn.execute(sql, params)
				.context("Failed to execute the query.")?;
			Ok(())
		})
		.await?;
		Ok(())
	}

	/// Retrieve the memoized output from a previous evaluation of an expression, if one exists on this server.
	pub async fn get_local_memoized_evaluation(
		self: &Arc<Self>,
		expression_hash: &expression::Hash,
	) -> Result<Option<Expression>> {
		let output = self
			.database_transaction(|txn| {
				let sql = r#"
					select
						output
					from
						evaluations
					where
						expression_hash = $1
				"#;
				let params = (expression_hash.to_string(),);
				let mut statement = txn
					.prepare_cached(sql)
					.context("Failed to prepare the query.")?;
				let expression: Option<Vec<u8>> = statement
					.query(params)
					.context("Failed to execute the query.")?
					.and_then(|row| row.get::<_, Vec<u8>>(0))
					.next()
					.transpose()
					.context("Failed to read a row from the query.")?;
				Ok(expression)
			})
			.await?;
		let output = if let Some(value) = output {
			let output = serde_json::from_slice(&value)?;
			Some(output)
		} else {
			None
		};
		Ok(output)
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

		let output = self.get_memoized_evaluation(expression_hash).await?;

		// If the output is None, return a 404
		let output = match output {
			Some(output) => output,
			None => return Ok(not_found()),
		};

		// Create the response.
		let body = serde_json::to_vec(&output).context("Failed to serialize the response body.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}
}
