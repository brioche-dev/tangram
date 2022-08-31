use super::{Client, Transport};
use crate::{artifact::Artifact, object::ObjectHash};
use anyhow::Result;

impl Client {
	pub async fn create_artifact(&self, object_hash: ObjectHash) -> Result<Artifact> {
		match &self.transport {
			Transport::InProcess(server) => {
				let artifact = server.create_artifact(object_hash).await?;
				Ok(artifact)
			},

			Transport::Unix(_) => todo!(),

			Transport::Tcp(transport) => {
				let path = format!("/artifacts/{object_hash}");
				let artifact = transport.post_json(&path, &object_hash).await?;
				Ok(artifact)
			},
		}
	}
}
