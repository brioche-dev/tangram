use super::Server;
use crate::artifact::Artifact;
use anyhow::{bail, Context, Result};
use std::sync::Arc;

impl Server {
	// Retrieve the artifact for a given package name and version.
	pub async fn get_package_version(
		self: &Arc<Self>,
		package_name: &str,
		package_version: &str,
	) -> Result<Artifact> {
		// Retrieve the artifact hash from the database.
		let object_hash = self
			.database_query_row(
				r#"
					select
						artifact_hash
					from package_versions
					where
						name = $1
					and
						version = $2
				"#,
				(package_name, package_version.to_string()),
				|row| Ok(row.get::<_, String>(0)?),
			)
			.await?
			.unwrap();

		// Construct the artifact.
		let object_hash = object_hash.parse().unwrap();
		let artifact = Artifact { object_hash };

		Ok(artifact)
	}

	// Create a new package version given an artifact.
	pub async fn create_package_version(
		self: &Arc<Self>,
		package_name: &str,
		package_version: &str,
		artifact: Artifact,
	) -> Result<Artifact> {
		// Create a new package version.
		self.database_execute(
			r#"
				replace into package_versions (
					name,
					version,
					artifact
				) values (
					$1,
					$2,
					$3
				)
			"#,
			(
				package_name,
				package_version,
				artifact.object_hash.to_string(),
			),
		)
		.await?;

		Ok(artifact)
	}
}

impl Server {
	// Retrieve the artifact for the given package name and version.
	pub async fn handle_get_package_version_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let (package_name, package_version) = if let ["packages", package_name, "versions", package_version] =
			path_components.as_slice()
		{
			(package_name, package_version)
		} else {
			bail!("Unexpected path.");
		};

		// Get the artifact.
		let artifact = self
			.get_package_version(package_name, package_version)
			.await?;

		// Create the response.
		let body =
			serde_json::to_vec(&artifact).context("Failed to serialize the response body.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}

	// Create a new package with the given package name, version, and artifact.
	pub async fn handle_create_package_version_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let (package_name, package_version) = if let &["packages", package_name, "versions", package_version] =
			path_components.as_slice()
		{
			(package_name.to_string(), package_version.to_string())
		} else {
			bail!("Unexpected path.");
		};

		// Read and deserialize the request body.
		let body = hyper::body::to_bytes(request.into_body())
			.await
			.context("Failed to read the request body.")?;
		let artifact =
			serde_json::from_slice(&body).context("Failed to deserialize the request body.")?;

		// Create the new package version.
		self.create_package_version(package_name.as_str(), package_version.as_str(), artifact)
			.await?;

		// Create the response.
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::empty())
			.unwrap();

		Ok(response)
	}
}
