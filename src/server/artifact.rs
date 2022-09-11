use super::{error::bad_request, Server};
use crate::{artifact::Artifact, object};
use anyhow::{bail, Context, Result};
use std::sync::Arc;

impl Server {
	// Create an artifact.
	pub async fn create_artifact(self: &Arc<Self>, object_hash: object::Hash) -> Result<Artifact> {
		self.database_transaction(|txn| {
			let sql = r#"
				replace into artifacts (
					object_hash
				) values (
					$1
				)
			"#;
			let params = (object_hash.to_string(),);
			txn.execute(sql, params)?;
			Ok(())
		})
		.await?;
		let artifact = Artifact::new(object_hash);
		Ok(artifact)
	}

	// Delete an artifact.
}

impl Server {
	pub(super) async fn handle_create_artifact_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let object_hash = if let ["artifacts", object_hash] = path_components.as_slice() {
			object_hash
		} else {
			bail!("Unexpected path.")
		};
		let object_hash = match object_hash.parse() {
			Ok(object_hash) => object_hash,
			Err(_) => return Ok(bad_request()),
		};

		// Create the artifact from the object hash.
		let artifact = self
			.create_artifact(object_hash)
			.await
			.context("Failed to create the artifact.")?;

		// Create the response.
		let body =
			serde_json::to_vec(&artifact).context("Failed to serialize the response body.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}
}
