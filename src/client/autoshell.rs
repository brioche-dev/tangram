use super::transport::InProcessOrHttp;
use crate::client::Client;
use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};

impl Client {
	pub async fn create_autoshell(&self, path: &Path) -> Result<()> {
		match self.transport.as_in_process_or_http() {
			InProcessOrHttp::InProcess(server) => {
				server.create_autoshell(path).await?;
				Ok(())
			},

			InProcessOrHttp::Http(http) => {
				http.post_json("/autoshells", &path).await?;
				Ok(())
			},
		}
	}

	pub async fn delete_autoshell(&self, path: &Path) -> Result<()> {
		match self.transport.as_in_process_or_http() {
			InProcessOrHttp::InProcess(server) => {
				server.delete_autoshell(path).await?;
				Ok(())
			},

			InProcessOrHttp::Http(http) => {
				let path = path.to_str().ok_or_else(|| anyhow!("Path must be utf-8"))?;
				// Build the URL.
				let mut url = http.base_url();
				url.set_path(&format!("/autoshells/${path}"));

				// Create the request.
				let request = http::Request::builder()
					.method(http::Method::DELETE)
					.uri(url.to_string())
					.header(http::header::CONTENT_TYPE, "application/json")
					.body(hyper::Body::empty())
					.unwrap();

				// Send the request.
				let response = http.request(request).await?;

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

				Ok(())
			},
		}
	}

	pub async fn get_autoshells(&self) -> Result<Vec<PathBuf>> {
		match self.transport.as_in_process_or_http() {
			InProcessOrHttp::InProcess(server) => {
				let paths = server.get_autoshells().await?;
				Ok(paths)
			},

			InProcessOrHttp::Http(http) => {
				let paths = http.get_json("/autoshells").await?;
				Ok(paths)
			},
		}
	}
}
