use crate::{id::Id, repl::Repl, server::Server};
use anyhow::{anyhow, Result};
use std::sync::Arc;

impl Server {
	pub async fn repl_new(self: &Arc<Server>) -> Result<Repl> {
		let id = Id::generate();
		let repl = Repl(id);
		let runtime = crate::server::runtime::js::Runtime::new(self);
		self.repls.lock().await.insert(id, runtime);
		Ok(repl)
	}

	pub async fn repl_run(self: &Arc<Server>, repl: &Repl, code: String) -> Result<Option<String>> {
		let repls = self.repls.lock().await;
		let runtime = repls
			.get(&repl.0)
			.ok_or_else(|| anyhow!(r#"Unable to find repl with id "{}"#, repl.0))?;
		let output = runtime.repl(code).await;
		Ok(output)
	}
}
