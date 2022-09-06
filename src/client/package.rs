use crate::{artifact::Artifact, client::Client, manifest::Manifest};
use anyhow::{bail, Context, Result};
use std::path::Path;

impl Client {
	pub async fn search(&self, name: &str) -> Result<Vec<String>> {
		match &self.transport {
			crate::client::transport::Transport::InProcess(server) => {
				let packages = server
					.get_packages(Some(name))
					.await?
					.into_iter()
					.map(|search_result| search_result.name)
					.collect();
				Ok(packages)
			},
			crate::client::transport::Transport::Unix(_) => todo!(),
			crate::client::transport::Transport::Tcp(transport) => {
				let path = "/packages";
				// Set the URL path.
				let mut url = transport.url.clone();
				url.set_path(path);
				url.set_query(Some(&format!("name={name}")));

				// Create the request.
				let request = http::Request::builder()
					.method(http::Method::GET)
					.uri(url.to_string())
					.body(hyper::Body::empty())
					.unwrap();

				// Send the request.
				let response = transport
					.client
					.request(request)
					.await
					.context("Failed to send the request.")?;

				// Handle a non-success status.
				if !response.status().is_success() {
					let status = response.status();
					let body = hyper::body::to_bytes(response.into_body())
						.await
						.context("Failed to read response body.")?;
					let body = String::from_utf8(body.to_vec())
						.context("Failed to read response body as string.")?;
					bail!("{}\n{}", status, body);
				}

				// Read the response body.
				let body = hyper::body::to_bytes(response.into_body())
					.await
					.context("Failed to read response body.")?;

				// Deserialize the response body.
				let response = serde_json::from_slice(&body)
					.context("Failed to deserialize the response body.")?;

				Ok(response)
			},
		}
	}
	pub async fn get_package(&self, name: &str, version: &str) -> Result<Option<Artifact>> {
		match self.transport.as_in_process_or_http() {
			super::transport::InProcessOrHttp::InProcess(server) => {
				let artifact = server.get_package_version(name, version).await?;
				Ok(artifact)
			},
			super::transport::InProcessOrHttp::Http(http) => {
				let path = format!("/packages/{name}/versions/{version}");
				let artifact = http.get_json(&path).await?;
				Ok(artifact)
			},
		}
	}

	pub async fn publish_package(&self, package_path: &Path) -> Result<Artifact> {
		// TODO.
		let locked = false;

		// Checkin the package.
		let package = self
			.checkin_package(package_path, locked)
			.await
			.context("Failed to check in package")?;

		// Read the manifest.
		let manifest_path = package_path.join("tangram.json");
		let manifest = tokio::fs::read(&manifest_path)
			.await
			.context("Failed to read the package manifest.")?;
		let manifest: Manifest = serde_json::from_slice(&manifest).with_context(|| {
			format!(
				r#"Failed to parse the package manifest at path "{}"."#,
				manifest_path.display()
			)
		})?;

		let name = manifest.name;
		let version = manifest.version;
		let artifact = package;

		// Create the package version.
		match self.transport.as_in_process_or_http() {
			super::transport::InProcessOrHttp::InProcess(server) => {
				let artifact = server
					.create_package_version(&name, &version, artifact)
					.await?;
				Ok(artifact)
			},
			super::transport::InProcessOrHttp::Http(http) => {
				let path = format!("/packages/{name}/versions/{version}");
				let artifact = http.post_json(&path, &artifact).await?;
				Ok(artifact)
			},
		}
	}
}
