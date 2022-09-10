use super::{transport::InProcessOrHttp, Client};
use crate::expression::Expression;
use anyhow::{Context, Result};

impl Client {
	pub async fn get_memoized_evaluation(
		&self,
		expression: &Expression,
	) -> Result<Option<Expression>> {
		match self.transport.as_in_process_or_http() {
			InProcessOrHttp::InProcess(server) => {
				let output = server.get_memoized_evaluation(expression).await?;
				Ok(output)
			},

			InProcessOrHttp::Http(http) => {
				// Compute the expression hash.
				let expression_hash = expression.hash();

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

				// If the server returns a 404, there is no memoized value for the expression.
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
