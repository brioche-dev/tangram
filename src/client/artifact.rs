use super::Client;
use crate::{artifact::Artifact, object::ObjectHash};
use anyhow::Result;

impl Client {
	pub async fn create_artifact(&self, object_hash: ObjectHash) -> Result<Artifact> {
		match self.transport.as_in_process_or_http() {
			super::transport::InProcessOrHttp::InProcess(server) => {
				let artifact = server.create_artifact(object_hash).await?;
				Ok(artifact)
			},
			super::transport::InProcessOrHttp::Http(http) => {
				let path = format!("/artifacts/{object_hash}");
				let artifact = http.post_json(&path, &object_hash).await?;
				Ok(artifact)
			},
		}
	}
}
