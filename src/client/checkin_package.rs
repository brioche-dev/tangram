use crate::artifact::ArtifactHash;
use crate::client::Client;
use anyhow::Result;
use std::path::Path;

impl Client {
	/// Checkin a package along with all its path dependencies.
	pub async fn checkin_package(&self, _path: impl AsRef<Path>) -> Result<ArtifactHash> {
		todo!()
	}
}
