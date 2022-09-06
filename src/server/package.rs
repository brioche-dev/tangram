use super::Server;
use crate::{artifact::Artifact, package::SearchResultItem};
use anyhow::{bail, Context, Result};
use std::sync::Arc;

#[derive(serde::Serialize)]
#[allow(clippy::module_name_repetitions)]
pub struct PackageVersion {
	version: String,
	artifact: Artifact,
}

impl Server {
	pub async fn get_packages(
		self: &Arc<Self>,
		name: Option<&str>,
	) -> Result<Vec<SearchResultItem>> {
		// Retrieve packages that match this query.
		let packages = if let Some(name) = name {
			self.database_query_rows(
				r#"
					select
					name
						from packages
					where name like $1
				"#,
				(format!("%{name}%"),),
				|row| row.get::<_, String>(0),
			)
			.await?
			.into_iter()
			.map(|name| SearchResultItem { name })
			.collect()
		} else {
			self.database_query_rows(
				r#"
					select
					name
						from packages
				"#,
				(),
				|row| row.get::<_, String>(0),
			)
			.await?
			.into_iter()
			.map(|name| SearchResultItem { name })
			.collect()
		};

		Ok(packages)
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
				let artifact = Artifact::new(object_hash);
				PackageVersion { version, artifact }
			})
			.collect();

		Ok(versions)
	}

	// Create a new package.
	pub async fn create_package(self: &Arc<Self>, package_name: &str) -> Result<()> {
		// Create the database connection.
		let database_connection_object = self
			.database_connection_pool
			.get()
			.await
			.context("Failed to retrieve a database connection.")?;

		tokio::task::block_in_place(move || -> Result<()> {
			// Create a database transaction.
			let mut database_connection = database_connection_object.lock().unwrap();
			let txn = database_connection.transaction()?;

			// Check if the package already exists.
			let sql = r#"
					select count(*) > 0 from packages where name = $1
				"#;
			let mut statement = txn
				.prepare_cached(sql)
				.context("Failed to prepare the query.")?;
			let package_exists = statement
				.query_row((package_name,), |row| row.get::<_, bool>(0))
				.context("Failed to execute the query.")?;

			drop(statement);

			if !package_exists {
				// Create the package.
				let sql = r#"
					insert into packages (
						name
					) values (
						$1
					)
				"#;
				let mut statement = txn
					.prepare_cached(sql)
					.context("Failed to prepare the query.")?;
				statement
					.execute((package_name,))
					.context("Failed to execute the query.")?;
			}

			txn.commit()?;

			Ok(())
		})?;

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
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Get the package versions.
		#[derive(serde::Deserialize, Default)]
		struct SearchParams {
			name: Option<String>,
		}
		let search_params: Option<SearchParams> = if let Some(query) = request.uri().query() {
			Some(serde_urlencoded::from_str(query)?)
		} else {
			None
		};
		let packages = self
			.get_packages(
				search_params
					.and_then(|search_params| search_params.name)
					.as_deref(),
			)
			.await?;

		// Create the response.
		let body = serde_json::to_vec(&packages).context("Failed to serialize the response.")?;
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
			Ok(_) => http::Response::builder()
				.status(http::StatusCode::OK)
				.body(hyper::Body::empty())
				.unwrap(),
			Err(err) => http::Response::builder()
				.status(http::StatusCode::BAD_REQUEST)
				.body(hyper::Body::from(err.to_string()))
				.unwrap(),
		};
		Ok(response)
	}
}
