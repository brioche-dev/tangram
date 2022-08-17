use super::runtime;
use crate::{id::Id, repl::ReplId, server::Server};
use anyhow::{anyhow, Result};
use std::sync::Arc;

pub struct Repl {
	pub runtime: runtime::js::Runtime,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct RunRequest {
	pub repl_id: ReplId,
	pub code: String,
}

impl Server {
	pub async fn create_repl(self: &Arc<Self>) -> Result<ReplId> {
		let id = Id::generate();
		let repl_id = ReplId(id);
		let runtime = crate::server::runtime::js::Runtime::new(self);
		let repl = Repl { runtime };
		self.repls.lock().await.insert(repl_id, repl);
		Ok(repl_id)
	}

	pub async fn repl_run(
		self: &Arc<Self>,
		repl_id: &ReplId,
		code: String,
	) -> Result<Result<Option<String>, String>> {
		let repls = self.repls.lock().await;
		let repl = repls
			.get(repl_id)
			.ok_or_else(|| anyhow!(r#"Unable to find a repl with id "{}"."#, repl_id))?;
		let result = repl.runtime.repl(code).await?;
		Ok(result)
	}
}
