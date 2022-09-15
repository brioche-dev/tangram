use crate::client::Client;
use anyhow::{bail, Context, Result};

impl Client {
	pub async fn garbage_collect(&self) -> Result<()> {
		match &self.transport.as_in_process_or_http() {
			super::transport::InProcessOrHttp::InProcess(server) => {
				server
					.garbage_collect()
					.await
					.context("Failed to garbage collect.")?;
				Ok(())
			},
			super::transport::InProcessOrHttp::Http(_) => {
				bail!("Cannot garbage collect a remote server.");
			},
		}
	}
}
