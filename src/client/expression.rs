use super::{Client, Transport};
use crate::{expression::Expression, hash::Hash, value::Value};
use anyhow::{Context, Result};

impl Client {
	pub async fn get_memoized_value_for_expression(
		&self,
		expression: &Expression,
	) -> Result<Option<Value>> {
		match &self.transport {
			Transport::InProcess(server) => {
				let value = server.get_memoized_value_for_expression(expression).await?;
				Ok(value)
			},
			Transport::Unix(_) => {
				todo!()
			},
			Transport::Tcp(transport) => {
				let expression_json = serde_json::to_vec(&expression)?;
				let expression_hash = Hash::new(&expression_json);
				// Set the URL path.
				let mut url = transport.url.clone();
				url.set_path(&format!("/expressions/{expression_hash}"));

				// Create the request.
				let request = http::Request::builder()
					.method(http::Method::GET)
					.uri(url.to_string())
					.body(hyper::Body::empty())
					.unwrap();

				// Send the request.
				let response = transport
					.client
					.request(request)
					.await
					.context("Failed to send the request.")?;

				if response.status() == http::StatusCode::NOT_FOUND {
					return Ok(None);
				}

				// Read the response body.
				let body = hyper::body::to_bytes(response.into_body())
					.await
					.context("Failed to read response body.")?;

				// Deserialize the response body.
				let value = serde_json::from_slice(&body)
					.context("Failed to deserialize the response body.")?;

				Ok(value)
			},
		}
	}
}
