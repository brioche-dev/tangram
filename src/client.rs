use crate::error::{Error, Result};
use crate::{build::Build, value, Value};
use crate::{id::Id, rid::Rid};
use futures::Stream;
use url::Url;

pub const API_URL: &str = "https://api.tangram.dev";

/// A client.
pub struct Client {
	url: Url,
	token: std::sync::RwLock<Option<String>>,
	client: reqwest::Client,
}

pub struct Run {
	id: Id,
	build: Id,
	state: State,
}
pub enum State {
	Running { children: Vec<Rid> },
	Complete { log: Id, children: Vec<Rid> },
}

pub struct Log(Vec<Event>);
pub enum Event {
	Stdout(Vec<u8>),
	Stderr(Vec<u8>),
	Child(Rid),
	Output(Option<Value>),
}

impl Client {
	#[must_use]
	pub fn new(url: Url, token: Option<String>) -> Client {
		let client = reqwest::Client::builder()
			.pool_max_idle_per_host(16)
			.build()
			.unwrap();
		Client {
			url,
			token: std::sync::RwLock::new(token),
			client,
		}
	}

	pub fn set_token(&self, token: Option<String>) {
		*self.token.write().unwrap() = token;
	}

	pub fn request(&self, method: reqwest::Method, url: Url) -> reqwest::RequestBuilder {
		let mut request = self.client.request(method, url);
		if let Some(token) = self.token.read().unwrap().as_ref() {
			request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"));
		}
		request
	}

	// HEAD /v1/values/<ID>
	pub async fn head(&self, id: Id) -> Result<bool> {
		todo!()
	}

	// GET /v1/values/<ID>
	pub async fn get(&self, id: Id) -> Result<value::Data> {
		todo!()
	}

	// PUT /v1/values/<ID>/
	pub async fn put(&self, value: value::Data) -> Result<Id> {
		todo!()
	}

	// GET /v1/builds/<ID>/output
	pub async fn output(&self, id: Id) -> Result<Option<value::Data>> {
		todo!()
	}

	// POST /v1/builds/<ID>/run
	pub async fn run(&self, id: Id) -> Result<Run> {
		todo!()
	}

	// GET /v1/runs/<ID>
	pub async fn get_run(&self, id: Rid) -> Result<Run> {
		todo!()
	}

	// GET /v1/runs/<ID>/log
	pub async fn get_log(&self, id: Rid) -> Result<Box<dyn Stream<Item = Event>>> {
		todo!()
	}

	// // POST /v1/command
	// pub async fn run_command(&self, command: Command) -> Result<tokio::net::TcpStream> {
	// 	todo!()
	// }
}
