use super::{
	error::bad_request,
	runtime::{
		self,
		js::{self},
	},
};
use crate::{id::Id, repl, server::Server};
use anyhow::{anyhow, bail, Context, Result};
use std::sync::Arc;

pub struct Repl {
	pub runtime: runtime::js::Runtime,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum Output {
	#[serde(rename = "success")]
	Success { message: Option<String> },
	#[serde(rename = "error")]
	Error { message: String },
}

impl Server {
	pub async fn create_repl(self: &Arc<Self>) -> Result<repl::Id> {
		let id = Id::generate();
		let repl_id = repl::Id(id);
		let runtime = js::Runtime::new(self);
		let repl = Repl { runtime };
		self.repls.lock().await.insert(repl_id, repl);
		Ok(repl_id)
	}

	pub async fn repl_run(self: &Arc<Self>, repl_id: repl::Id, code: String) -> Result<Output> {
		let repls = self.repls.lock().await;
		let repl = repls
			.get(&repl_id)
			.ok_or_else(|| anyhow!(r#"Unable to find a repl with id "{}"."#, repl_id))?;
		let output = repl.runtime.repl(code).await?;
		Ok(output)
	}
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CreateResponse {
	pub id: repl::Id,
}

impl Server {
	pub(super) async fn handle_create_repl_request(
		self: &Arc<Self>,
		_request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Create a repl.
		let repl_id = self
			.create_repl()
			.await
			.context("Failed to create a REPL.")?;

		// Create the response.
		let response = CreateResponse { id: repl_id };
		let body = serde_json::to_vec(&response)?;
		let response = http::Response::builder()
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct RunRequest {
	pub code: String,
}

pub type RunResponse = Output;

impl Server {
	pub(super) async fn handle_repl_run_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let repl_id = if let ["repls", repl_id, "run"] = path_components.as_slice() {
			repl_id.to_owned()
		} else {
			bail!("Unexpected path.");
		};
		let repl_id: repl::Id = match repl_id.parse() {
			Ok(repl_id) => repl_id,
			Err(_) => return Ok(bad_request()),
		};

		// Read the code to run from the request body.
		let body = hyper::body::to_bytes(request)
			.await
			.context("Failed to read request body.")?;
		let RunRequest { code } =
			serde_json::from_slice(&body).context("Failed to deserialize repl run request.")?;

		// Run the repl.
		let output = self.repl_run(repl_id, code).await?;

		// Create the response.
		let body = serde_json::to_vec(&output)?;
		let response = http::Response::builder()
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}
}
