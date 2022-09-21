use super::Server;
use crate::{hash::Hash, package::SearchResultItem};
use anyhow::{bail, Context, Result};
use std::sync::Arc;

#[derive(serde::Serialize)]
pub struct Version {
	version: String,
	artifact: Hash,
}

impl Server {
	pub async fn search_packages(self: &Arc<Self>, name: &str) -> Result<Vec<SearchResultItem>> {
		// Retrieve packages that match this query.
		let packages = self
			.database_transaction(|txn| {
				let sql = r#"
					select
						name
					from
						packages
					where
						name like $1
				"#;
				let params = (format!("%{name}%"),);
				let mut statement = txn
					.prepare_cached(sql)
					.context("Failed to prepare the query.")?;
				let items = statement
					.query(params)
					.context("Failed to exeucte the query.")?
					.and_then(|row| {
						let name = row.get::<_, String>(0)?;
						let item = SearchResultItem { name };
						Ok(item)
					})
					.collect::<Result<_>>()?;
				Ok(items)
			})
			.await?;
		Ok(packages)
	}

	pub async fn get_packages(self: &Arc<Self>) -> Result<Vec<SearchResultItem>> {
		// Retrieve packages that match this query.
		let packages = self
			.database_transaction(|txn| {
				let sql = r#"
					select
						name
					from
						packages
				"#;
				let mut statement = txn
					.prepare_cached(sql)
					.context("Failed to prepare the query.")?;
				let items = statement
					.query(())
					.context("Failed to execute the query.")?
					.and_then(|row| {
						let name = row.get::<_, String>(0)?;
						let item = SearchResultItem { name };
						Ok(item)
					})
					.collect::<Result<_>>()?;
				Ok(items)
			})
			.await?;
		Ok(packages)
	}

	pub async fn get_package(self: &Arc<Self>, package_name: &str) -> Result<Vec<Version>> {
		// Retrieve the package versions.
		let versions = self
			.database_transaction(|txn| {
				let sql = r#"
					select
						version,
						hash
					from
						package_versions
					where
						name = $1
				"#;
				let params = (package_name,);
				let mut statement = txn
					.prepare_cached(sql)
					.context("Failed to prepare the query.")?;
				let versions = statement
					.query(params)
					.context("Failed to execute the query.")?
					.and_then(|row| {
						let version = row.get::<_, String>(0)?;
						let hash = row.get::<_, String>(1)?;
						let hash = hash.parse().with_context(|| "Failed to parse the hash.")?;
						let package_version = Version {
							version,
							artifact: hash,
						};
						Ok(package_version)
					})
					.collect::<Result<_>>()?;
				Ok(versions)
			})
			.await?;

		Ok(versions)
	}

	// Create a new package.
	pub async fn create_package(self: &Arc<Self>, package_name: &str) -> Result<()> {
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

			if !package_exists {
				// Create the package.
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

			Ok(())
		})
		.await?;

		Ok(())
	}
}

#[derive(serde::Serialize)]
pub struct GetPackageResponse {
	versions: Vec<Version>,
}

impl Server {
	// Retrieve the packages name list.
	pub(super) async fn handle_get_packages_request(
		self: &Arc<Self>,
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
			self.search_packages(name).await?
		} else {
			self.get_packages().await?
		};

		// Create the response.
		let body = serde_json::to_vec(&packages).context("Failed to serialize the response.")?;
		let response = http::Response::builder()
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}
}

impl Server {
	// Retrieve the package versions for the given package name.
	pub(super) async fn handle_get_package_request(
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
	pub(super) async fn handle_create_package_request(
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

		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(hyper::Body::empty())
			.unwrap();

		Ok(response)
	}
}
