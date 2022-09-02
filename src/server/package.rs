use super::Server;
use crate::artifact::Artifact;
use anyhow::{anyhow, bail, Context, Result};
use std::sync::Arc;

#[derive(serde::Serialize)]
#[allow(clippy::module_name_repetitions)]
pub struct PackageVersion {
	version: String,
	artifact: Artifact,
}

impl Server {
	pub async fn get_packages(self: &Arc<Self>) -> Result<Vec<String>> {
		// Retrieve the package versions.
		let versions = self
			.database_query_rows(
				r#"
					select
					name
						from packages
				"#,
				(),
				|row| Ok(row.get::<_, String>(0)?),
			)
			.await?
			.into_iter()
			.collect();

		Ok(versions)
	}

	pub async fn get_package(self: &Arc<Self>, package_name: &str) -> Result<Vec<PackageVersion>> {
		// Retrieve the package versions.
		let versions = self
			.database_query_rows(
				r#"
					select
						version,
						artifact_hash
					from package_versions
					where
						name = $1
				"#,
				(package_name,),
				|row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
			)
			.await?
			.into_iter()
			.map(|(version, object_hash)| {
				let object_hash = object_hash
					.parse()
					.with_context(|| "Failed to parse object hash.")
					.unwrap();
				let artifact = Artifact { object_hash };
				PackageVersion { version, artifact }
			})
			.collect();

		Ok(versions)
	}

	// Create a new package.
	pub async fn create_package(self: &Arc<Self>, package_name: &str) -> Result<()> {
		// TODO combine into single transaction.
		// Check if a package with this name already exists.
		let package_exists = self
			.database_query_row(
				r#"
					select count(*) > 0 from packages where name = $1
				"#,
				(package_name,),
				|row| Ok(row.get::<_, bool>(0)?),
			)
			.await?
			.unwrap();

		if package_exists {
			return Err(anyhow!(format!(
				"Package with name '{package_name}' already exists."
			)));
		}

		self.database_execute(
			r#"
				insert into packages (
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
	// Retrieve the packages name list.
	pub async fn handle_get_packages_request(
		self: &Arc<Self>,
		_request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Get the package versions.
		let versions = self.get_packages().await?;

		// Create the response.
		let body = serde_json::to_vec(&versions).context("Failed to serialize the response.")?;
		let response = http::Response::builder()
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}

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
		let create_package_result = self.create_package(package_name).await;

		// Create the response.
		let response = match create_package_result {
			Ok(_) => {
				http::Response::builder()
					.status(http::StatusCode::OK)
					.body(hyper::Body::empty())
					.unwrap()
			},
			Err(err) => {
				http::Response::builder()
					.status(http::StatusCode::BAD_REQUEST)
					.body(hyper::Body::from(err.to_string()))
					.unwrap()
			},
		};
		Ok(response)
	}
}
