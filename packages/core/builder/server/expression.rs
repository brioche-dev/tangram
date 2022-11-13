use super::{error::bad_request, Server};
use crate::expression::{AddExpressionOutcome, Expression};
use anyhow::{bail, Context, Result};

pub type AddExpressionRequest = Expression;

pub type AddExpressionResponse = AddExpressionOutcome;

impl Server {
	pub(super) async fn handle_add_expression_request(
		&self,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read and deserialize the request body.
		let body = hyper::body::to_bytes(request.into_body())
			.await
			.context("Failed to read the request body.")?;
		let expression =
			serde_json::from_slice(&body).context("Failed to deserialize the request body.")?;

		// Add the expression.
		let outcome = self
			.builder
			.lock_shared()
			.await?
			.try_add_expression(&expression)
			.await
			.context("Failed to get the expression.")?;

		// Create the response.
		let body =
			serde_json::to_vec(&outcome).context("Failed to serialize the response body.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}

	pub(super) async fn handle_get_expression_request(
		&self,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let hash = if let ["expressions", hash] = path_components.as_slice() {
			hash
		} else {
			bail!("Unexpected path.");
		};
		let hash = match hash.parse() {
			Ok(hash) => hash,
			Err(_) => return Ok(bad_request()),
		};

		// Get the expression.
		let expression = self
			.builder
			.lock_shared()
			.await?
			.try_get_expression_local(hash)?;

		// Create the response.
		let body =
			serde_json::to_vec(&expression).context("Failed to serialize the response body.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}
}
