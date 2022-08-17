use crate::{artifact::Artifact, client::Client, server::Server, util::path_exists};
use anyhow::Result;
use std::{path::PathBuf, sync::Arc};

pub struct Fragment {
	server: Arc<Server>,
	artifact: Artifact,
}

impl Fragment {
	#[must_use]
	pub fn artifact(&self) -> &Artifact {
		&self.artifact
	}

	#[must_use]
	pub fn path(&self) -> PathBuf {
		self.server
			.path
			.join("fragments")
			.join(self.artifact().to_string())
	}
}

impl Server {
	pub(super) async fn create_fragment(self: &Arc<Self>, artifact: &Artifact) -> Result<Fragment> {
		// Get the path to the fragment.
		let fragment_path = self.path.join("fragments").join(artifact.to_string());

		let client = Client::new_in_process(Arc::clone(self));

		// If the fragment path does not exist, then checkout the object to the fragment path.
		if !path_exists(&fragment_path).await? {
			client.checkout(artifact, &fragment_path).await?;
		}

		// Create the fragment.
		let fragment = Fragment {
			server: Arc::clone(self),
			artifact: artifact.clone(),
		};

		Ok(fragment)
	}
}
