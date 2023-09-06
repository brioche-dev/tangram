use crate::{Id, Result, Server};
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
pub enum Kind {
	Direct(Server),
	Remote {
		url: Url,
		token: std::sync::RwLock<Option<String>>,
		client: reqwest::Client,
	},
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
		let kind = Kind::Remote {
			url,
			token: std::sync::RwLock::new(token),
			client,
		};
		Self::with_kind(kind)
	}

	pub fn with_kind(kind: Kind) -> Self {
		let state = State {
			file_descriptor_semaphore: tokio::sync::Semaphore::new(16),
			kind,
		};
		Self {
			state: Arc::new(state),
		}
	}

	// pub fn set_token(&self, token: Option<String>) {
	// 	*self.state.token.write().unwrap() = token;
	// }

	#[must_use]
	pub fn file_descriptor_semaphore(&self) -> &tokio::sync::Semaphore {
		&self.state.file_descriptor_semaphore
	}

	// pub fn request(&self, method: reqwest::Method, url: Url) -> reqwest::RequestBuilder {
	// 	let mut request = self.state.client.request(method, url);
	// 	if let Some(token) = self.state.token.read().unwrap().as_ref() {
	// 		request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"));
	// 	}
	// 	request
	// }

	pub async fn value_exists(&self, id: Id) -> Result<bool> {
		match &self.state.kind {
			Kind::Direct(server) => server.value_exists(id).await,
			Kind::Remote { .. } => todo!(),
		}
	}

	pub async fn try_get_value_bytes(&self, id: Id) -> Result<Option<Vec<u8>>> {
		match &self.state.kind {
			Kind::Direct(server) => server.try_get_value_bytes(id).await,
			Kind::Remote { .. } => todo!(),
		}
	}

	pub async fn try_put_value(&self, id: Id, bytes: &[u8]) -> Result<Result<(), Vec<Id>>> {
		match &self.state.kind {
			Kind::Direct(server) => server.try_put_value_bytes(id, bytes).await,
			Kind::Remote { .. } => todo!(),
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
