use crate::{evaluation, return_error, Error, Result, Server, WrapErr};
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
	pub async fn get_value_exists(&self, id: crate::Id) -> Result<bool> {
		match self {
			Self::Server(server) => server.get_value_exists(id).await,
			Self::Reqwest(client) => client.get_value_exists(id).await,
		}
	}

	#[async_recursion]
	pub async fn try_get_value_bytes(&self, id: crate::Id) -> Result<Option<Vec<u8>>> {
		match self {
			Self::Server(server) => server.try_get_value_bytes(id).await,
			Self::Reqwest(client) => client.try_get_value_bytes(id).await,
		}
	}

	#[async_recursion]
	pub async fn try_put_value_bytes(
		&self,
		id: crate::Id,
		bytes: &[u8],
	) -> Result<Result<(), Vec<crate::Id>>> {
		match self {
			Self::Server(server) => server.try_put_value_bytes(id, bytes).await,
			Self::Reqwest(client) => client.try_put_value_bytes(id, bytes).await,
		}
	}

	#[async_recursion]
	pub async fn try_get_assignment(&self, id: crate::Id) -> Result<Option<evaluation::Id>> {
		match self {
			Self::Server(server) => server.try_get_assignment(id).await,
			Self::Reqwest(_) => todo!(),
		}
	}

	#[async_recursion]
	pub async fn evaluate(&self, id: crate::Id) -> Result<evaluation::Id> {
		match self {
			Self::Server(server) => server.evaluate(id).await,
			Self::Reqwest(_) => todo!(),
		}
	}

	#[async_recursion]
	pub async fn try_get_evaluation_bytes(&self, id: evaluation::Id) -> Result<Option<Vec<u8>>> {
		match self {
			Self::Server(server) => server.try_get_evaluation_bytes(id).await,
			Self::Reqwest(_) => todo!(),
		}
	}

	#[async_recursion]
	pub async fn try_get_evaluation_children(
		&self,
		id: evaluation::Id,
	) -> Result<Option<BoxStream<evaluation::Id>>> {
		match self {
			Self::Server(server) => server.try_get_evaluation_children(id).await,
			Self::Reqwest(_) => todo!(),
		}
	}

	#[async_recursion]
	pub async fn try_get_evaluation_log(
		&self,
		id: evaluation::Id,
	) -> Result<Option<BoxStream<Vec<u8>>>> {
		match self {
			Self::Server(server) => server.try_get_evaluation_log(id).await,
			Self::Reqwest(_) => todo!(),
		}
	}

	#[async_recursion]
	pub async fn get_evaluation_result(
		&self,
		id: evaluation::Id,
	) -> Result<evaluation::Result<crate::Id>> {
		self.try_get_evaluation_result(id)
			.await?
			.wrap_err("Expected the evaluation to exist.")
	}

	#[async_recursion]
	pub async fn try_get_evaluation_result(
		&self,
		id: evaluation::Id,
	) -> Result<Option<evaluation::Result<crate::Id>>> {
		match self {
			Self::Server(server) => server.try_get_evaluation_result(id).await,
			Self::Reqwest(_) => todo!(),
		}
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

	pub async fn get_value_exists(&self, id: crate::Id) -> Result<bool> {
		let request = self.request(http::Method::HEAD, &format!("/v1/values/{id}"));
		let response = request.send().await?;
		match response.status() {
			http::StatusCode::OK => Ok(true),
			http::StatusCode::NOT_FOUND => Ok(false),
			_ => return_error!(r#"Unexpected status code "{}"."#, response.status()),
		}
	}

	pub async fn try_get_value_bytes(&self, id: crate::Id) -> Result<Option<Vec<u8>>> {
		let request = self.request(http::Method::GET, &format!("/v1/values/{id}"));
		let response = request.send().await?;
		match response.status() {
			http::StatusCode::OK => {},
			http::StatusCode::NOT_FOUND => return Ok(None),
			_ => return_error!(r#"Unexpected status code "{}"."#, response.status()),
		};
		let bytes = response.bytes().await?;
		Ok(Some(bytes.into()))
	}

	pub async fn try_put_value_bytes(
		&self,
		id: crate::Id,
		bytes: &[u8],
	) -> Result<Result<(), Vec<crate::Id>>> {
		let request = self
			.request(http::Method::PUT, &format!("/v1/values/{id}"))
			.body(bytes.to_owned());
		let response = request.send().await?;
		match response.status() {
			http::StatusCode::OK => Ok(Ok(())),
			http::StatusCode::BAD_REQUEST => {
				let bytes = response.bytes().await?;
				let missing_children = serde_json::from_slice(&bytes).map_err(Error::other)?;
				Ok(Err(missing_children))
			},
			_ => return_error!(r#"Unexpected status code "{}"."#, response.status()),
		}
	}
}
