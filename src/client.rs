use crate::{return_error, Error, Id, Result, Server};
use std::sync::Arc;
use url::Url;

pub const API_URL: &str = "https://api.tangram.dev";

/// A client.
#[derive(Clone, Debug)]
pub struct Client {
	state: Arc<State>,
}

#[derive(Debug)]
struct State {
	kind: Kind,
	file_descriptor_semaphore: tokio::sync::Semaphore,
}

#[derive(Debug)]
enum Kind {
	Direct(Server),
	Remote(Remote),
}

#[derive(Debug)]
struct Remote {
	url: Url,
	token: std::sync::RwLock<Option<String>>,
	client: reqwest::Client,
}

impl Client {
	#[must_use]
	pub fn new_direct(server: Server) -> Self {
		Self::with_kind(Kind::Direct(server))
	}

	#[must_use]
	pub fn new_remote(url: Url, token: Option<String>) -> Self {
		let client = reqwest::Client::builder()
			.pool_max_idle_per_host(16)
			.build()
			.unwrap();
		let kind = Kind::Remote(Remote {
			url,
			token: std::sync::RwLock::new(token),
			client,
		});
		Self::with_kind(kind)
	}

	fn with_kind(kind: Kind) -> Self {
		let state = State {
			file_descriptor_semaphore: tokio::sync::Semaphore::new(16),
			kind,
		};
		Self {
			state: Arc::new(state),
		}
	}

	pub fn set_token(&self, token: Option<String>) {
		if let Kind::Remote(remote) = &self.state.kind {
			*remote.token.write().unwrap() = token;
		}
	}

	#[must_use]
	pub fn file_descriptor_semaphore(&self) -> &tokio::sync::Semaphore {
		&self.state.file_descriptor_semaphore
	}

	pub async fn value_exists(&self, id: Id) -> Result<bool> {
		match &self.state.kind {
			Kind::Direct(server) => server.value_exists(id).await,
			Kind::Remote(remote) => {
				let request = remote.request(http::Method::HEAD, &format!("/v1/values/{id}"));
				let response = request.send().await?;
				match response.status() {
					http::StatusCode::OK => Ok(true),
					http::StatusCode::NOT_FOUND => Ok(false),
					_ => return_error!(r#"Unexpected status code "{}"."#, response.status()),
				}
			},
		}
	}

	pub async fn try_get_value_bytes(&self, id: Id) -> Result<Option<Vec<u8>>> {
		match &self.state.kind {
			Kind::Direct(server) => server.try_get_value_bytes(id).await,
			Kind::Remote(remote) => {
				let request = remote.request(http::Method::GET, &format!("/v1/values/{id}"));
				let response = request.send().await?;
				match response.status() {
					http::StatusCode::OK => {},
					http::StatusCode::NOT_FOUND => return Ok(None),
					_ => return_error!(r#"Unexpected status code "{}"."#, response.status()),
				};
				let bytes = response.bytes().await?;
				Ok(Some(bytes.into()))
			},
		}
	}

	pub async fn try_put_value(&self, id: Id, bytes: &[u8]) -> Result<Result<(), Vec<Id>>> {
		match &self.state.kind {
			Kind::Direct(server) => server.try_put_value_bytes(id, bytes).await,
			Kind::Remote(remote) => {
				let request = remote
					.request(http::Method::PUT, &format!("/v1/values/{id}"))
					.body(bytes.to_owned());
				let response = request.send().await?;
				match response.status() {
					http::StatusCode::OK => Ok(Ok(())),
					http::StatusCode::BAD_REQUEST => {
						let bytes = response.bytes().await?;
						let missing_children =
							serde_json::from_slice(&bytes).map_err(Error::other)?;
						Ok(Err(missing_children))
					},
					_ => return_error!(r#"Unexpected status code "{}"."#, response.status()),
				}
			},
		}
	}

	// // GET /v1/builds/<ID>/output
	// pub async fn output(&self, id: Id) -> Result<Option<value::Data>> {
	// 	todo!()
	// }

	// // POST /v1/builds/<ID>/run
	// pub async fn run(&self, id: Id) -> Result<Run> {
	// 	todo!()
	// }

	// // GET /v1/runs/<ID>
	// pub async fn get_run(&self, id: Rid) -> Result<Run> {
	// 	todo!()
	// }

	// // GET /v1/runs/<ID>/log
	// pub async fn get_log(&self, id: Rid) -> Result<Box<dyn Stream<Item = Event>>> {
	// 	todo!()
	// }

	// // POST /v1/command
	// pub async fn run_command(&self, command: Command) -> Result<tokio::net::TcpStream> {
	// 	todo!()
	// }
}

impl Remote {
	pub fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
		let url = format!("{}{}", self.url, path.strip_prefix('/').unwrap());
		let mut request = self.client.request(method, url);
		if let Some(token) = self.token.read().unwrap().as_ref() {
			request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"));
		}
		request
	}
}
