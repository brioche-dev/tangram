use super::Client;
use crate::{
	expression::{AddExpressionOutcome, Expression},
	hash::Hash,
};
use anyhow::{bail, Context, Result};

impl Client {
	pub async fn get_expression(&self, hash: Hash) -> Result<Expression> {
		let path = format!("/expressions/{}", hash);
		let outcome = self.get_json(&path).await?;
		Ok(outcome)
	}

	pub async fn add_expression(&self, expression: &Expression) -> Result<Hash> {
		match self.try_add_expression(expression).await? {
			AddExpressionOutcome::Added { hash } => Ok(hash),
			_ => bail!("Failed to add the expression."),
		}
	}

	pub async fn try_add_expression(
		&self,
		expression: &Expression,
	) -> Result<AddExpressionOutcome> {
		let outcome = self.post_json("/expressions/", expression).await?;
		Ok(outcome)
	}

	pub async fn get_memoized_evaluation(
		&self,
		expression_hash: Hash,
	) -> Result<Option<Expression>> {
		// Build the URL.
		let mut url = self.url.clone();
		url.set_path(&format!("/expressions/{expression_hash}"));

		// Create the request.
		let request = http::Request::builder()
			.method(http::Method::GET)
			.uri(url.to_string())
			.body(hyper::Body::empty())
			.unwrap();

		// Send the request.
		let response = self
			.request(request)
			.await
			.context("Failed to send the request.")?;

		// If the server returns a 404, there is no memoized evaluation of the expression.
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}

		// Read the response body.
		let body = hyper::body::to_bytes(response.into_body())
			.await
			.context("Failed to read the response body.")?;

		// Deserialize the response body.
		let output =
			serde_json::from_slice(&body).context("Failed to deserialize the response body.")?;

		Ok(output)
	}
}
