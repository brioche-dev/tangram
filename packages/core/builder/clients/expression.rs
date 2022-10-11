use crate::{
	expression::{AddExpressionOutcome, Expression},
	hash::Hash,
};
use anyhow::{bail, Context, Result};
use url::Url;

#[derive(Clone)]
pub struct Client {
	pub url: Url,
	pub token: Option<String>,
	pub http_client: reqwest::Client,
}

impl Client {
	#[must_use]
	pub fn new(url: Url, token: Option<String>) -> Client {
		let http_client = reqwest::Client::new();
		Client {
			url,
			token,
			http_client,
		}
	}
}

impl Client {
	pub fn request(&self, method: reqwest::Method, url: Url) -> reqwest::RequestBuilder {
		let mut request = self.http_client.request(method, url);
		if let Some(token) = &self.token {
			request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"));
		}
		request
	}
}

impl Client {
	pub async fn try_get_expression(&self, hash: Hash) -> Result<Option<Expression>> {
		let response = self
			.try_get_expression_with_output(hash)
			.await?
			.map(|(expression, _)| expression);
		Ok(response)
	}

	pub async fn try_get_expression_with_output(
		&self,
		hash: Hash,
	) -> Result<Option<(Expression, Option<Hash>)>> {
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
			.request(http::Method::POST, url)
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
