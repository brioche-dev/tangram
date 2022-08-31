use super::Server;
use crate::artifact::Artifact;
use anyhow::{bail, Context, Result};
use std::sync::Arc;

#[derive(serde::Serialize)]
pub struct PackageVersion {
	version: String,
	artifact: Artifact,
}

impl Server {
	async fn get_package(self: &Arc<Self>, package_name: &str) -> Result<Vec<PackageVersion>> {
		// Retrieve the package versions.
		let versions = self
			.database_query_rows(
				r#"
					select
						version,
						artifact
					from package_versions
					where
						name = $1
				"#,
				(package_name,),
				|row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?)),
			)
			.await?
			.into_iter()
			.map(|(version, artifact)| {
				let artifact = serde_json::from_slice(&artifact)
					.context("Failed to deserialize artifact.")
					.unwrap();
				PackageVersion { version, artifact }
			})
			.collect();

		Ok(versions)
	}

	// Create a new package.
	async fn create_package(self: &Arc<Self>, package_name: &str) -> Result<()> {
		self.database_execute(
			r#"
				replace into packages (
					name
				) values (
					$1
				)
			"#,
			(package_name,),
		)
		.await?;

		Ok(())
	}
}

#[derive(serde::Serialize)]
pub struct GetPackageResponse {
	versions: Vec<PackageVersion>,
}

impl Server {
	// Retrieve the package versions for the given package name.
	pub async fn handle_get_package_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let package_name = if let &["packages", package_name] = path_components.as_slice() {
			package_name
		} else {
			bail!("Unexpected path.");
		};

		// Get the package versions.
		let versions = self.get_package(package_name).await?;

		// Create the response.
		let response = GetPackageResponse { versions };
		let body = serde_json::to_vec(&response).context("Failed to serialize the response.")?;
		let response = http::Response::builder()
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}
}

impl Server {
	// Create a package with the given name.
	pub async fn handle_create_package_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let package_name = if let &["packages", package_name] = path_components.as_slice() {
			package_name
		} else {
			bail!("Unexpected path.");
		};

		// Create the package.
		self.create_package(package_name).await?;

		// Create the response.
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::empty())
			.unwrap();

		Ok(response)
	}
}
