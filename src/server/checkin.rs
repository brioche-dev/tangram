use super::Server;
use crate::{client::Client, hash::Hash};
use anyhow::Result;
use std::{path::Path, sync::Arc};

impl Server {
	pub(super) async fn checkin(self: &Arc<Self>, path: &Path) -> Result<Hash> {
		// Create a client for this server to perform the checkin.
		let client = Client::new_for_server(self);

		// Perform the checkin.
		let artifact = client.checkin(path).await?;

		Ok(artifact)
	}
}
