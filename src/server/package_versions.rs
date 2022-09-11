use super::{error::not_found, Server};
use crate::artifact::Artifact;
use anyhow::{anyhow, bail, Context, Result};
use std::sync::Arc;

impl Server {
	// Retrieve the artifact for a given package name and version.
	pub async fn get_package_version(
		self: &Arc<Self>,
		package_name: &str,
		package_version: &str,
	) -> Result<Option<Artifact>> {
		// Retrieve the artifact hash from the database.
		let maybe_object_hash = self
			.database_transaction(|txn| {
				let sql = r#"
					select
						artifact_hash
					from
						package_versions
					where
						name = $1 and version = $2
				"#;
				let params = (package_name, package_version.to_string());
				let mut statement = txn
					.prepare_cached(sql)
					.context("Failed to prepare the query.")?;
				let maybe_object_hash = statement
					.query(params)
					.context("Failed to execute the query.")?
					.and_then(|row| row.get::<_, String>(0))
					.next()
					.transpose()?;
				Ok(maybe_object_hash)
			})
			.await?;

		// Construct the artifact.
		let artifact =
			maybe_object_hash.map(|object_hash| Artifact::new(object_hash.parse().unwrap()));

		Ok(artifact)
	}

	// Create a new package version given an artifact.
	pub async fn create_package_version(
		self: &Arc<Self>,
		package_name: &str,
		package_version: &str,
		artifact: Artifact,
	) -> Result<Artifact> {
		self.database_transaction(|txn| {
			// Check if the package already exists.
			let sql = r#"
				select
					count(*) > 0
				from
					packages
				where
					name = $1
			"#;
			let params = (package_name,);
			let mut statement = txn
				.prepare_cached(sql)
				.context("Failed to prepare the query.")?;
			let package_exists = statement
				.query(params)
				.context("Failed to execute the query.")?
				.and_then(|row| row.get::<_, bool>(0))
				.next()
				.transpose()?
				.unwrap();

			// Create the package if it does not exist.
			if !package_exists {
				let sql = r#"
					insert into packages (
						name
					) values (
						$1
					)
				"#;
				let params = (package_name,);
				let mut statement = txn
					.prepare_cached(sql)
					.context("Failed to prepare the query.")?;
				statement
					.execute(params)
					.context("Failed to execute the query.")?;
			}

			// Check if the package version already exists.
			let sql = r#"
				select
					count(*) > 0
				from
					package_versions
				where
					name = $1 and version = $2
			"#;
			let params = (package_name, package_version);
			let mut statement = txn
				.prepare_cached(sql)
				.context("Failed to prepare the query.")?;
			let package_version_exists = statement
				.query(params)
				.context("Failed to execute the query.")?
				.and_then(|row| row.get::<_, bool>(0))
				.next()
				.transpose()?
				.unwrap();

			if package_version_exists {
				return Err(anyhow!(format!(
					r#"The package with name "{package_name}" and version "{package_version}" already exists."#
				)));
			}

			// Create the new package version.
			let sql = r#"
				insert into package_versions (
					name, version, artifact_hash
				) values (
					$1, $2, $3
				)
			"#;
			let params = (
				package_name,
				package_version,
				artifact.object_hash().to_string(),
			);
			let mut statement = txn
				.prepare_cached(sql)
				.context("Failed to prepare the query.")?;
			statement
				.execute(params)
				.context("Failed to execute the query.")?;
			drop(statement);

			Ok(())
		})
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
		let body =
			serde_json::to_vec(&artifact).context("Failed to serialize the response body.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::from(body))
			.unwrap();
		Ok(response)
	}
}
