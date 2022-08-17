use super::{Client, Transport};
use crate::{repl::ReplId, server::repl::RunRequest};
use anyhow::Result;

impl Client {
	pub async fn create_repl(&self) -> Result<ReplId> {
		if let Transport::InProcess { server, .. } = &self.transport {
			server.create_repl().await
		} else {
			self.post("/repls/").await
		}
	}

	pub async fn repl_run(
		&self,
		repl_id: ReplId,
		code: &str,
	) -> Result<Result<Option<String>, String>> {
		if let Transport::InProcess { server, .. } = &self.transport {
			server.repl_run(&repl_id, code.to_string()).await
		} else {
			let request = RunRequest {
				repl_id,
				code: code.to_owned(),
			};
			self.post_json("/repl_run", &request).await
		}
	}
}
