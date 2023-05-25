use crate::{
	error::Result,
	operation,
	util::task_map::TaskMap,
	value::Value,
};

use std::{
	sync::Arc,
};

pub struct Run {
    pub(crate) http_client: reqwest::Client,
    pub(crate) task_map:
        std::sync::Mutex<Option<Arc<TaskMap<operation::Hash, Result<Value>>>>>,
}

impl Run {
    pub fn new () -> Run {
        // Create the HTTP client.
		let http_client = reqwest::Client::new();

		// Create the operations task map.
		let task_map = std::sync::Mutex::new(None);

        Run { http_client, task_map }
    }
}
