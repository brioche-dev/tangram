use self::{
	config::Config,
	transport::{InProcessOrHttp, Transport},
};
use crate::{heuristics::FILESYSTEM_CONCURRENCY_LIMIT, server::Server};
use anyhow::Result;
use async_recursion::async_recursion;
use std::sync::Arc;
use tokio::sync::Semaphore;

mod artifact;
pub mod checkin;
mod checkin_package;
pub mod checkout;
pub mod config;
mod evaluate;
mod expression;
mod gc;
mod object_cache;
mod package;
mod repl;
mod transport;

pub struct Client {
	transport: Transport,
	file_system_semaphore: Arc<Semaphore>,
}

impl Client {
	#[async_recursion]
	#[must_use]
	pub async fn new_with_config(config: Config) -> Result<Client> {
		let transport = match config.transport {
			self::config::Transport::InProcess { server } => {
				let server = Server::new(server).await?;
				Transport::InProcess(server)
			},
			self::config::Transport::Unix { path } => Transport::Unix(transport::Unix::new(path)),
			self::config::Transport::Tcp { url } => Transport::Tcp(transport::Tcp::new(url)),
		};

		let file_system_semaphore = Arc::new(Semaphore::new(FILESYSTEM_CONCURRENCY_LIMIT));

		let client = Client {
			transport,
			file_system_semaphore,
		};

		Ok(client)
	}

	#[must_use]
	pub fn new_for_server(server: &Arc<Server>) -> Client {
		let transport = Transport::InProcess(Arc::clone(server));
		let file_system_semaphore = Arc::new(Semaphore::new(FILESYSTEM_CONCURRENCY_LIMIT));
		Client {
			transport,
			file_system_semaphore,
		}
	}

	#[must_use]
	pub fn as_in_process(&self) -> Option<&Arc<Server>> {
		match self.transport.as_in_process_or_http() {
			InProcessOrHttp::InProcess(server) => Some(server),
			InProcessOrHttp::Http(_) => None,
		}
	}
}
