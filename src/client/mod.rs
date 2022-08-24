use crate::{heuristics::FILESYSTEM_CONCURRENCY_LIMIT, server::Server};
use anyhow::{bail, Result};
use hyperlocal::UnixClientExt;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Semaphore;
use url::Url;

mod artifact;
pub mod checkin;
mod checkin_package;
pub mod checkout;
mod evaluate;
mod object_cache;
mod repl;

pub struct Client {
	transport: Transport,
	file_system_semaphore: Arc<Semaphore>,
}

pub enum Transport {
	InProcess {
		server: Arc<Server>,
	},
	Unix {
		path: PathBuf,
		client: hyper::Client<hyperlocal::UnixConnector, hyper::Body>,
	},
	Tcp {
		url: Url,
		client:
			hyper::Client<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>, hyper::Body>,
	},
}

impl Client {
	pub fn new(transport: Transport) -> Client {
		let file_system_semaphore = Arc::new(Semaphore::new(FILESYSTEM_CONCURRENCY_LIMIT));
		Client {
			transport,
			file_system_semaphore,
		}
	}

	#[must_use]
	pub fn new_in_process(server: Arc<Server>) -> Client {
		let transport = Transport::InProcess { server };
		Client::new(transport)
	}

	#[must_use]
	pub fn new_unix(path: PathBuf) -> Client {
		let client = hyper::Client::unix();
		let transport = Transport::Unix { path, client };
		Client::new(transport)
	}

	#[must_use]
	pub fn new_tcp(url: Url) -> Client {
		let client: hyper::Client<_, hyper::Body> = hyper::Client::builder().build(
			hyper_rustls::HttpsConnectorBuilder::new()
				.with_native_roots()
				.https_or_http()
				.enable_http1()
				.build(),
		);
		let file_system_semaphore = Arc::new(Semaphore::new(FILESYSTEM_CONCURRENCY_LIMIT));
		let transport = Transport::Tcp { url, client };
		Client {
			transport,
			file_system_semaphore,
		}
	}

	async fn post_json<T, U>(&self, path: &str, body: &T) -> Result<U>
	where
		T: serde::Serialize,
		U: serde::de::DeserializeOwned,
	{
		let body = serde_json::to_string(&body)?;
		let body = hyper::Body::from(body);
		let response = match &self.transport {
			Transport::InProcess { .. } => {
				bail!("Cannot perform request with in process client.");
			},
			Transport::Unix {
				path: unix_path,
				client,
				..
			} => {
				let uri = hyperlocal::Uri::new(unix_path, "/evaluate");
				let request = http::Request::builder()
					.method(http::Method::POST)
					.header(http::header::CONTENT_TYPE, "application/json")
					.uri(uri)
					.body(body)
					.unwrap();
				client.request(request).await?
			},
			Transport::Tcp { url, client, .. } => {
				let mut url = url.clone();
				url.set_path(path);
				let request = http::Request::builder()
					.method(http::Method::POST)
					.header(http::header::CONTENT_TYPE, "application/json")
					.uri(url.to_string())
					.body(body)
					.unwrap();
				client.request(request).await?
			},
		};
		if !response.status().is_success() {
			let status = response.status();
			bail!("{}", status);
		}
		let body = hyper::body::to_bytes(response.into_body()).await?;
		let response = serde_json::from_slice(&body)?;
		Ok(response)
	}

	async fn post<U>(&self, path: &str) -> Result<U>
	where
		U: serde::de::DeserializeOwned,
	{
		let response = match &self.transport {
			Transport::InProcess { .. } => {
				bail!("Cannot perform request with in process client.");
			},
			Transport::Unix {
				path: unix_path,
				client,
				..
			} => {
				let uri = hyperlocal::Uri::new(unix_path, "/evaluate");
				let request = http::Request::builder()
					.method(http::Method::POST)
					.uri(uri)
					.body(hyper::Body::empty())
					.unwrap();
				client.request(request).await?
			},
			Transport::Tcp { url, client, .. } => {
				let mut url = url.clone();
				url.set_path(path);
				let request = http::Request::builder()
					.method(http::Method::POST)
					.uri(url.to_string())
					.body(hyper::Body::empty())
					.unwrap();
				client.request(request).await?
			},
		};
		if !response.status().is_success() {
			let status = response.status();
			bail!("{}", status);
		}
		let body = hyper::body::to_bytes(response.into_body()).await?;
		let response = serde_json::from_slice(&body)?;
		Ok(response)
	}
}
