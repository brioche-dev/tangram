use super::Server;
use crate::{artifact::Artifact, client::Client};
use anyhow::Result;
use std::{path::Path, sync::Arc};

impl Server {
	pub(super) async fn checkin(self: &Arc<Self>, path: &Path) -> Result<Artifact> {
		// Create a client to this server to perform the checkin.
		let client = Client::new_in_process(Arc::clone(self));
		let artifact = client.checkin(path).await?;
		Ok(artifact)
	}
}
