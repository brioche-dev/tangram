use crate::{error::Result, operation, util::task_map::TaskMap, value::Value};

use std::sync::Arc;

pub struct State {
	pub(crate) http_client: reqwest::Client,
	pub(crate) task_map: std::sync::Mutex<Option<Arc<TaskMap<operation::Hash, Result<Value>>>>>,
}

impl State {
	pub fn new() -> State {
		// Create the HTTP client.
		let http_client = reqwest::Client::new();

		// Create the operations task map.
		let task_map = std::sync::Mutex::new(None);

		State {
			http_client,
			task_map,
		}
	}
}
