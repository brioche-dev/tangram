use super::{error::bad_request, Server};
use crate::hash::Hash;
use anyhow::{bail, Context, Result};

impl Server {
	pub(super) async fn handle_evaluate_expression_request(
		&self,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let hash = if let ["expressions", hash, "evaluate"] = path_components.as_slice() {
			hash
		} else {
			bail!("Unexpected path.")
		};
		let hash: Hash = match hash.parse() {
			Ok(hash) => hash,
			Err(_) => return Ok(bad_request()),
		};

		// Evaluate the expression.
		let output = self
			.builder
			.lock_shared()
			.await?
			.evaluate(hash, hash)
			.await
			.context("Failed to evaluate the expression.")?;

		// Create the response.
		let body = serde_json::to_vec(&output)?;
		let response = http::Response::builder()
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}
}
