use super::{transport::InProcessOrHttp, Client};
use crate::{expression::Expression, hash::Hash, server::expression::AddExpressionOutcome};
use anyhow::{bail, Context, Result};

impl Client {
	pub async fn get_expression(&self, hash: Hash) -> Result<Expression> {
		match self.transport.as_in_process_or_http() {
			super::transport::InProcessOrHttp::InProcess(server) => {
				let outcome = server.get_expression(hash).await?;
				Ok(outcome)
			},
			super::transport::InProcessOrHttp::Http(http) => {
				let path = format!("/expressions/{}", hash);
				let outcome = http.get_json(&path).await?;
				Ok(outcome)
			},
		}
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
		match self.transport.as_in_process_or_http() {
			super::transport::InProcessOrHttp::InProcess(server) => {
				let outcome = server.try_add_expression(expression).await?;
				Ok(outcome)
			},
			super::transport::InProcessOrHttp::Http(http) => {
				let outcome = http.post_json("/expressions/", expression).await?;
				Ok(outcome)
			},
		}
	}

	pub async fn get_memoized_evaluation(
		&self,
		expression_hash: Hash,
	) -> Result<Option<Expression>> {
		match self.transport.as_in_process_or_http() {
			InProcessOrHttp::InProcess(server) => {
				let output = server.get_memoized_evaluation(expression_hash).await?;
				Ok(output)
			},

			InProcessOrHttp::Http(http) => {
				// Build the URL.
				let mut url = http.base_url();
				url.set_path(&format!("/expressions/{expression_hash}"));

				// Create the request.
				let request = http::Request::builder()
					.method(http::Method::GET)
					.uri(url.to_string())
					.body(hyper::Body::empty())
					.unwrap();

				// Send the request.
				let response = http
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
					.context("Failed to read response body.")?;

				// Deserialize the response body.
				let output = serde_json::from_slice(&body)
					.context("Failed to deserialize the response body.")?;

				Ok(output)
			},
		}
	}
}
