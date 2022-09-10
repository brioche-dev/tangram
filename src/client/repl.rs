use super::Client;
use crate::{repl, server};
use anyhow::{Context, Result};

impl Client {
	pub async fn create_repl(&self) -> Result<repl::Id> {
		match self.transport.as_in_process_or_http() {
			super::transport::InProcessOrHttp::InProcess(server) => {
				let id = server.create_repl().await?;
				Ok(id)
			},
			super::transport::InProcessOrHttp::Http(http) => {
				let body = http.post("/repls/", hyper::Body::empty()).await?;
				let body = hyper::body::to_bytes(body)
					.await
					.context("Failed to read request body.")?;
				let server::repl::CreateResponse { id } = serde_json::from_slice(&body)
					.context("Failed to deserialize the request body.")?;
				Ok(id)
			},
		}
	}

	pub async fn repl_run(&self, repl_id: repl::Id, code: String) -> Result<server::repl::Output> {
		match self.transport.as_in_process_or_http() {
			super::transport::InProcessOrHttp::InProcess(server) => {
				let output = server.repl_run(repl_id, code).await?;
				Ok(output)
			},
			super::transport::InProcessOrHttp::Http(http) => {
				let path = format!("/repls/{}/run", repl_id);
				let request = server::repl::RunRequest { code };
				let response = http.post_json(&path, &request).await?;
				Ok(response)
			},
		}
	}
}
