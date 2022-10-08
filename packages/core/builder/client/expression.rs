use super::Client;
use crate::{
	expression::{AddExpressionOutcome, Expression},
	hash::Hash,
};
use anyhow::{bail, Context, Result};

impl Client {
	pub async fn get_expression(&self, hash: Hash) -> Result<Expression> {
		let path = format!("/v1/expressions/{}", hash);

		// Build the URL.
		let mut url = self.url.clone();
		url.set_path(&path);

		// Send the request.
		let response = self
			.request(http::Method::GET, url)
			.send()
			.await?
			.error_for_status()?;

		// Read the response body.
		let response = response
			.json()
			.await
			.context("Failed to read the response body.")?;

		Ok(response)
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
		// Build the URL.
		let mut url = self.url.clone();
		url.set_path("/v1/expressions/");

		// Send the request.
		let response = self
			.http_client
			.request(http::Method::POST, url.to_string())
			.json(&expression)
			.send()
			.await?
			.error_for_status()?;

		// Read the response body.
		let response = response
			.json()
			.await
			.context("Failed to read the response body.")?;

		Ok(response)
	}

	pub async fn get_memoized_evaluation(
		&self,
		expression_hash: Hash,
	) -> Result<Option<Expression>> {
		// Build the URL.
		let mut url = self.url.clone();
		url.set_path(&format!("/v1/expressions/{expression_hash}"));

		// Send the request.
		let response = self
			.request(http::Method::GET, url)
			.send()
			.await
			.context("Failed to send the request.")?;

		// If the server returns a 404, there is no memoized evaluation of the expression.
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}

		// Read the response body.
		let body = response
			.bytes()
			.await
			.context("Failed to read the response body.")?;

		// Deserialize the response body.
		let output =
			serde_json::from_slice(&body).context("Failed to deserialize the response body.")?;

		Ok(output)
	}
}
