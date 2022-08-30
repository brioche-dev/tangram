use self::transport::Transport;
use crate::{heuristics::FILESYSTEM_CONCURRENCY_LIMIT, server::Server};
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
mod transport;

pub struct Client {
	transport: Transport,
	file_system_semaphore: Arc<Semaphore>,
}

impl Client {
	#[must_use]
	pub fn new(transport: Transport) -> Client {
		let file_system_semaphore = Arc::new(Semaphore::new(FILESYSTEM_CONCURRENCY_LIMIT));
		Client {
			transport,
			file_system_semaphore,
		}
	}

	#[must_use]
	pub fn new_in_process(server: Arc<Server>) -> Client {
		let transport = Transport::InProcess(server);
		Client::new(transport)
	}

	#[must_use]
	pub fn new_unix(path: PathBuf) -> Client {
		let transport = Transport::Unix(transport::Unix::new(path));
		Client::new(transport)
	}

	#[must_use]
	pub fn new_tcp(url: Url) -> Client {
		let transport = Transport::Tcp(transport::Tcp::new(url));
		Client::new(transport)
	}
}
