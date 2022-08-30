use crate::{artifact::Artifact, server::Server};
use std::{path::PathBuf, sync::Arc};

pub struct Fragment {
	pub(crate) server: Arc<Server>,
	pub(crate) artifact: Artifact,
}

impl Fragment {
	#[must_use]
	pub fn artifact(&self) -> &Artifact {
		&self.artifact
	}

	#[must_use]
	pub fn path(&self) -> PathBuf {
		self.server
			.path()
			.join("fragments")
			.join(self.artifact().to_string())
	}
}
