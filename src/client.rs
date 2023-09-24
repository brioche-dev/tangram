use crate::{object, return_error, run, task, Error, Result, Server, Value, WrapErr};
use async_recursion::async_recursion;
use futures::stream::BoxStream;
use std::sync::Arc;
use url::Url;

/// A client.
#[derive(Clone, Debug)]
pub enum Client {
	Server(Server),
	// Hyper(Hyper),
	Reqwest(Reqwest),
}

impl Client {
	#[must_use]
	pub fn with_server(server: Server) -> Self {
		Self::Server(server)
	}

	#[must_use]
	pub fn with_url(url: Url, token: Option<String>) -> Self {
		Self::Reqwest(Reqwest::new(url, token))
	}

	#[must_use]
	pub fn file_descriptor_semaphore(&self) -> &tokio::sync::Semaphore {
		match self {
			Self::Server(server) => &server.state.file_descriptor_semaphore,
			Self::Reqwest(client) => &client.state.file_descriptor_semaphore,
		}
	}

	#[async_recursion]
	pub async fn get_object_exists(&self, id: object::Id) -> Result<bool> {
		match self {
			Self::Server(server) => server.get_object_exists(id).await,
			Self::Reqwest(client) => client.get_object_exists(id).await,
		}
	}

	#[async_recursion]
	pub async fn try_get_object_bytes(&self, id: object::Id) -> Result<Option<Vec<u8>>> {
		match self {
			Self::Server(server) => server.try_get_object_bytes(id).await,
			Self::Reqwest(client) => client.try_get_object_bytes(id).await,
		}
	}

	#[async_recursion]
	pub async fn try_put_object_bytes(
		&self,
		id: object::Id,
		bytes: &[u8],
	) -> Result<Result<(), Vec<object::Id>>> {
		match self {
			Self::Server(server) => server.try_put_object_bytes(id, bytes).await,
			Self::Reqwest(client) => client.try_put_object_bytes(id, bytes).await,
		}
	}

	#[async_recursion]
	pub async fn run(&self, id: task::Id) -> Result<run::Id> {
		match self {
			Self::Server(server) => Ok(server.run(id).await?.id()),
			Self::Reqwest(_) => todo!(),
		}
	}

	#[async_recursion]
	pub async fn try_get_assignment(&self, id: task::Id) -> Result<Option<run::Id>> {
		match self {
			Self::Server(server) => server.try_get_assignment(id).await,
			Self::Reqwest(_) => todo!(),
		}
	}

	#[async_recursion]
	pub async fn try_get_run_bytes(&self, id: run::Id) -> Result<Option<Vec<u8>>> {
		match self {
			Self::Server(server) => server.try_get_run_bytes(id).await,
			Self::Reqwest(_) => todo!(),
		}
	}

	#[async_recursion]
	pub async fn try_get_run_children(
		&self,
		id: run::Id,
	) -> Result<Option<BoxStream<'static, run::Id>>> {
		match self {
			Self::Server(server) => server.try_get_run_children(id).await,
			Self::Reqwest(_) => todo!(),
		}
	}

	#[async_recursion]
	pub async fn try_get_run_log(
		&self,
		id: run::Id,
	) -> Result<Option<BoxStream<'static, Vec<u8>>>> {
		match self {
			Self::Server(server) => server.try_get_run_log(id).await,
			Self::Reqwest(_) => todo!(),
		}
	}

	#[async_recursion]
	pub async fn try_get_run_result(&self, id: run::Id) -> Result<Option<Result<Value>>> {
		match self {
			Self::Server(server) => server.try_get_run_result(id).await,
			Self::Reqwest(_) => todo!(),
		}
	}
}

impl Client {
	#[async_recursion]
	pub async fn get_run_children(&self, id: run::Id) -> Result<BoxStream<'static, run::Id>> {
		self.try_get_run_children(id)
			.await?
			.wrap_err("Expected the run to exist.")
	}

	#[async_recursion]
	pub async fn get_run_log(&self, id: run::Id) -> Result<BoxStream<'static, Vec<u8>>> {
		self.try_get_run_log(id)
			.await?
			.wrap_err("Expected the run to exist.")
	}

	#[async_recursion]
	pub async fn get_run_result(&self, id: run::Id) -> Result<Result<Value>> {
		self.try_get_run_result(id)
			.await?
			.wrap_err("Expected the run to exist.")
	}
}

#[derive(Clone, Debug)]
pub struct Reqwest {
	state: Arc<State>,
}

#[derive(Debug)]
struct State {
	client: reqwest::Client,
	file_descriptor_semaphore: tokio::sync::Semaphore,
	token: std::sync::RwLock<Option<String>>,
	url: Url,
}

impl Reqwest {
	#[must_use]
	pub fn new(url: Url, token: Option<String>) -> Self {
		let client = reqwest::Client::builder()
			.http2_prior_knowledge()
			.build()
			.unwrap();
		let state = State {
			url,
			token: std::sync::RwLock::new(token),
			client,
			file_descriptor_semaphore: tokio::sync::Semaphore::new(16),
		};
		Self {
			state: Arc::new(state),
		}
	}

	pub fn set_token(&self, token: Option<String>) {
		*self.state.token.write().unwrap() = token;
	}

	fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
		let url = format!("{}{}", self.state.url, path.strip_prefix('/').unwrap());
		let mut request = self.state.client.request(method, url);
		if let Some(token) = self.state.token.read().unwrap().as_ref() {
			request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"));
		}
		request
	}

	pub async fn get_object_exists(&self, id: object::Id) -> Result<bool> {
		let request = self.request(http::Method::HEAD, &format!("/v1/objects/{id}"));
		let response = request.send().await?;
		match response.status() {
			http::StatusCode::OK => Ok(true),
			http::StatusCode::NOT_FOUND => Ok(false),
			_ => return_error!(r#"Unexpected status code "{}"."#, response.status()),
		}
	}

	pub async fn try_get_object_bytes(&self, id: object::Id) -> Result<Option<Vec<u8>>> {
		let request = self.request(http::Method::GET, &format!("/v1/objects/{id}"));
		let response = request.send().await?;
		match response.status() {
			http::StatusCode::OK => {},
			http::StatusCode::NOT_FOUND => return Ok(None),
			_ => return_error!(r#"Unexpected status code "{}"."#, response.status()),
		};
		let bytes = response.bytes().await?;
		Ok(Some(bytes.into()))
	}

	pub async fn try_put_object_bytes(
		&self,
		id: object::Id,
		bytes: &[u8],
	) -> Result<Result<(), Vec<object::Id>>> {
		let request = self
			.request(http::Method::PUT, &format!("/v1/objects/{id}"))
			.body(bytes.to_owned());
		let response = request.send().await?;
		match response.status() {
			http::StatusCode::OK => Ok(Ok(())),
			http::StatusCode::BAD_REQUEST => {
				let bytes = response.bytes().await?;
				let missing_children =
					tangram_serialize::from_slice(&bytes).map_err(Error::other)?;
				Ok(Err(missing_children))
			},
			_ => return_error!(r#"Unexpected status code "{}"."#, response.status()),
		}
	}
}
