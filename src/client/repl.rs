use super::{Client, Transport};
use crate::{repl::ReplId, server};
use anyhow::{Context, Result};

impl Client {
	pub async fn create_repl(&self) -> Result<ReplId> {
		match &self.transport {
			Transport::InProcess(server) => {
				let id = server.create_repl().await?;
				Ok(id)
			},

			Transport::Unix(transport) => {
				let body = transport.post("/repls/", hyper::Body::empty()).await?;
				let body = hyper::body::to_bytes(body)
					.await
					.context("Failed to read request body.")?;
				let server::repl::CreateResponse { id } = serde_json::from_slice(&body)
					.context("Failed to deserialize the request body.")?;
				Ok(id)
			},

			Transport::Tcp(transport) => {
				let body = transport.post("/repls/", hyper::Body::empty()).await?;
				let body = hyper::body::to_bytes(body)
					.await
					.context("Failed to read request body.")?;
				let server::repl::CreateResponse { id } = serde_json::from_slice(&body)
					.context("Failed to deserialize the request body.")?;
				Ok(id)
			},
		}
	}

	pub async fn repl_run(&self, repl_id: ReplId, code: String) -> Result<server::repl::Output> {
		match &self.transport {
			Transport::InProcess(server) => {
				let output = server.repl_run(repl_id, code).await?;
				Ok(output)
			},

			Transport::Unix(transport) => {
				let path = format!("/repls/{}/run", repl_id);
				let request = server::repl::RunRequest { code };
				let response = transport.post_json(&path, &request).await?;
				Ok(response)
			},

			Transport::Tcp(transport) => {
				let path = format!("/repls/{}/run", repl_id);
				let request = server::repl::RunRequest { code };
				let response = transport.post_json(&path, &request).await?;
				Ok(response)
			},
		}
	}
}
