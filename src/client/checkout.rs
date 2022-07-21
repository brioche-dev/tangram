use crate::artifact::ArtifactHash;
use crate::client::Client;
use anyhow::Result;
use std::path::Path;

impl Client {
	pub async fn checkout(
		&self,
		_artifact_hash: ArtifactHash,
		_path: impl AsRef<Path>,
	) -> Result<()> {
		todo!()
	}
}
