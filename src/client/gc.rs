use crate::client::Client;
use anyhow::{Context, Result};

impl Client {
	pub async fn garbage_collect(&self) -> Result<()> {
		match &self.transport {
			crate::client::transport::Transport::InProcess(server) => {
				server
					.garbage_collect()
					.await
					.context("Failed to garbage collect.")?;
				Ok(())
			},
			crate::client::transport::Transport::Unix(_) => todo!(),
			crate::client::transport::Transport::Tcp(_) => {
				todo!()
			},
		}
	}
}
