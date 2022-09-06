use super::Server;
use crate::{
	artifact::Artifact,
	client::{checkout::ExternalPathForDependencyFn, Client},
};
use anyhow::Result;
use std::{path::Path, sync::Arc};

impl Server {
	pub(super) async fn checkout(
		self: &Arc<Self>,
		artifact: Artifact,
		path: &Path,
		external_path_for_dependency: Option<&'_ ExternalPathForDependencyFn>,
	) -> Result<()> {
		// Create a client for this server to perform the checkin.
		let client = Client::new_for_server(self);

		// Perform the checkout.
		client
			.checkout(artifact, path, external_path_for_dependency)
			.await?;

		Ok(())
	}
}
