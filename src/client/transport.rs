use crate::server::Server;
use anyhow::{bail, Context, Result};
use hyperlocal::UnixClientExt;
use std::{path::PathBuf, sync::Arc};
use url::Url;

pub enum Transport {
	InProcess(Arc<Server>),
	Unix(Unix),
	Tcp(Tcp),
}

pub struct Unix {
	path: PathBuf,
	client: hyper::Client<hyperlocal::UnixConnector, hyper::Body>,
}

impl Unix {
	pub fn new(path: PathBuf) -> Unix {
		let client = hyper::Client::unix();
		Unix { path, client }
	}

	pub async fn post(&self, path: &str, body: hyper::Body) -> Result<hyper::Body> {
		let uri = hyperlocal::Uri::new(&self.path, path);
		todo!()
	}

	pub async fn post_json<T, U>(&self, path: &str, body: &T) -> Result<U>
	where
		T: serde::Serialize,
		U: serde::de::DeserializeOwned,
	{
		todo!()
	}
}

pub struct Tcp {
	url: Url,
	client: hyper::Client<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>, hyper::Body>,
}

impl Tcp {
	pub fn new(url: Url) -> Tcp {
		let client: hyper::Client<_, hyper::Body> = hyper::Client::builder().build(
			hyper_rustls::HttpsConnectorBuilder::new()
				.with_native_roots()
				.https_or_http()
				.enable_http1()
				.build(),
		);
		Tcp { url, client }
	}

	pub async fn post(&self, path: &str, body: hyper::Body) -> Result<hyper::Body> {
		// Set the URL path.
		let mut url = self.url.clone();
		url.set_path(path);

		// Create the request.
		let request = http::Request::builder()
			.method(http::Method::POST)
			.uri(url.to_string())
			.body(body)
			.unwrap();

		// Send the request.
		let response = self
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

		Ok(response.into_body())
	}

	pub async fn post_json<T, U>(&self, path: &str, body: &T) -> Result<U>
	where
		T: serde::Serialize,
		U: serde::de::DeserializeOwned,
	{
		// Set the URL path.
		let mut url = self.url.clone();
		url.set_path(path);

		// Serialize the body.
		let body = serde_json::to_string(&body).context("Failed to serialize the request body.")?;

		// Create the request.
		let request = http::Request::builder()
			.method(http::Method::POST)
			.uri(url.to_string())
			.header(http::header::CONTENT_TYPE, "application/json")
			.body(hyper::Body::from(body))
			.unwrap();

		// Send the request.
		let response = self
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
		let response =
			serde_json::from_slice(&body).context("Failed to deserialize the response body.")?;

		Ok(response)
	}
}
