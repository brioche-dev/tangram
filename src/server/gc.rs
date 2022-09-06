use super::Server;
use anyhow::Result;
use std::sync::Arc;

impl Server {
	// Add an object to the server after ensuring the server has all its references.
	pub async fn garbage_collect(self: &Arc<Self>) -> Result<()> {
		Ok(())
	}
}
