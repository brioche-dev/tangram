use crate::package::Version;

use super::{error::not_found, Server};
use anyhow::{bail, Context, Result};

impl Server {
	// Retrieve the packages name list.
	pub(super) async fn handle_get_packages_request(
		&self,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the search params.
		#[derive(serde::Deserialize, Default)]
		struct SearchParams {
			name: Option<String>,
		}
		let search_params: Option<SearchParams> = if let Some(query) = request.uri().query() {
			Some(serde_urlencoded::from_str(query)?)
		} else {
			None
		};

		let packages = if let Some(name) = search_params
			.as_ref()
			.and_then(|search_params| search_params.name.as_deref())
		{
			self.builder
				.lock_shared()
				.await?
				.search_packages(name)
				.await?
		} else {
			self.builder.lock_shared().await?.get_packages().await?
		};

		// Create the response.
		let body = serde_json::to_vec(&packages).context("Failed to serialize the response.")?;
		let response = http::Response::builder()
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}
}

#[derive(serde::Serialize)]
pub struct GetPackageResponse {
	versions: Vec<Version>,
}

impl Server {
	// Retrieve the package versions for the given package name.
	pub(super) async fn handle_get_package_request(
		&self,
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
		let versions = self
			.builder
			.lock_shared()
			.await?
			.get_package(package_name)
			.await?;

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
	pub(super) async fn handle_create_package_request(
		&self,
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
		self.builder
			.lock_shared()
			.await?
			.create_package(package_name)
			.await?;

		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::empty())
			.unwrap();

		Ok(response)
	}
}

impl Server {
	// Retrieve the artifact for the given package name and version.
	pub async fn handle_get_package_version_request(
		&self,
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
			.builder
			.lock_shared()
			.await?
			.get_package_version(package_name, package_version)
			.await?;

		// Create the response.
		let response = match artifact {
			Some(artifact) => {
				let body = serde_json::to_vec(&artifact)
					.context("Failed to serialize the response body.")?;
				http::Response::builder()
					.status(http::StatusCode::OK)
					.body(hyper::Body::from(body))
					.unwrap()
			},
			None => not_found(),
		};

		Ok(response)
	}

	// Create a new package with the given package name, version, and artifact.
	pub async fn handle_create_package_version_request(
		&self,
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
		self.builder
			.lock_shared()
			.await?
			.create_package_version(package_name.as_str(), package_version.as_str(), artifact)
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
}
