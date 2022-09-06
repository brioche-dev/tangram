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
	pub path: PathBuf,
	pub client: hyper::Client<hyperlocal::UnixConnector, hyper::Body>,
}

pub struct Tcp {
	pub url: Url,
	pub client:
		hyper::Client<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>, hyper::Body>,
}

pub enum Http<'a> {
	Unix(&'a Unix),
	Tcp(&'a Tcp),
}

pub enum InProcessOrHttp<'a> {
	InProcess(&'a Arc<Server>),
	Http(Http<'a>),
}

impl Transport {
	pub fn as_http(&self) -> Option<Http<'_>> {
		match self {
			Transport::InProcess(_) => None,
			Transport::Unix(unix) => Some(Http::Unix(unix)),
			Transport::Tcp(tcp) => Some(Http::Tcp(tcp)),
		}
	}

	pub fn as_in_process_or_http(&self) -> InProcessOrHttp<'_> {
		match self {
			Transport::InProcess(server) => InProcessOrHttp::InProcess(server),
			Transport::Unix(unix) => InProcessOrHttp::Http(Http::Unix(unix)),
			Transport::Tcp(tcp) => InProcessOrHttp::Http(Http::Tcp(tcp)),
		}
	}
}

impl Unix {
	pub fn new(path: PathBuf) -> Unix {
		let client = hyper::Client::unix();
		Unix { path, client }
	}
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
}

impl<'a> Http<'a> {
	pub fn base_url(&self) -> Url {
		match self {
			Http::Unix(unix) => {
				let uri: hyper::Uri = hyperlocal::Uri::new(&unix.path, "/").into();
				uri.to_string().parse().unwrap()
			},
			Http::Tcp(tcp) => tcp.url.clone(),
		}
	}

	pub async fn request(
		&self,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		match self {
			Http::Unix(unix) => unix
				.client
				.request(request)
				.await
				.context("Failed to send the request."),
			Http::Tcp(tcp) => tcp
				.client
				.request(request)
				.await
				.context("Failed to send the request."),
		}
	}

	pub async fn get(&self, path: &str) -> Result<hyper::Body> {
		// Build the URL.
		let mut url = self.base_url();
		url.set_path(path);

		// Create the request.
		let request = http::Request::builder()
			.method(http::Method::GET)
			.uri(url.to_string())
			.body(hyper::Body::empty())
			.unwrap();

		// Send the request.
		let response = self.request(request).await?;

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

	pub async fn get_json<U>(&self, path: &str) -> Result<U>
	where
		U: serde::de::DeserializeOwned,
	{
		// Build the URL.
		let mut url = self.base_url();
		url.set_path(path);

		// Create the request.
		let request = http::Request::builder()
			.method(http::Method::GET)
			.uri(url.to_string())
			.body(hyper::Body::empty())
			.unwrap();

		// Send the request.
		let response = match self {
			Http::Unix(unix) => unix
				.client
				.request(request)
				.await
				.context("Failed to send the request.")?,
			Http::Tcp(tcp) => tcp
				.client
				.request(request)
				.await
				.context("Failed to send the request.")?,
		};

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

	pub async fn post(&self, path: &str, body: hyper::Body) -> Result<hyper::Body> {
		// Build the URL.
		let mut url = self.base_url();
		url.set_path(path);

		// Create the request.
		let request = http::Request::builder()
			.method(http::Method::POST)
			.uri(url.to_string())
			.body(body)
			.unwrap();

		// Send the request.
		let response = self.request(request).await?;

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
		// Build the URL.
		let mut url = self.base_url();
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
		let response = self.request(request).await?;

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
