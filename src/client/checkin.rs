use crate::{artifact::ArtifactHash, client::Client};
use anyhow::Result;
use std::path::Path;

impl Client {
	pub async fn checkin(&self, _path: impl AsRef<Path>) -> Result<ArtifactHash> {
		todo!()
	}
}
