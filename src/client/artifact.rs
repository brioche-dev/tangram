use super::{Client, Transport};
use crate::{artifact::Artifact, object::ObjectHash};
use anyhow::Result;

impl Client {
	pub async fn create_artifact(&self, object_hash: ObjectHash) -> Result<Artifact> {
		let artifact = match &self.transport {
			Transport::InProcess(server) => server.create_artifact(object_hash).await?,
			_ => todo!(),
		};
		Ok(artifact)
	}
}
