use super::Server;
use crate::{
	artifact::Artifact,
	client::{checkout::DependencyHandlerFn, Client},
};
use anyhow::Result;
use std::{path::Path, sync::Arc};

impl Server {
	pub(super) async fn checkout(
		self: &Arc<Self>,
		artifact: Artifact,
		path: &Path,
		dependency_handler: Option<&'_ DependencyHandlerFn>,
	) -> Result<()> {
		// Create a client for this server to perform the checkin.
		let client = Client::new_for_server(self);

		// Perform the checkout.
		client.checkout(artifact, path, dependency_handler).await?;

		Ok(())
	}
}
