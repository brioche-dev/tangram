use crate::{artifact::Artifact, client::Client};
use anyhow::Result;

impl Client {
	pub async fn get_package(&self, name: &str, version: &str) -> Result<Artifact> {
		match &self.transport {
			crate::client::transport::Transport::InProcess(server) => {
				let artifact = server.get_package_version(name, version).await?;
				Ok(artifact)
			},
			crate::client::transport::Transport::Unix(_) => todo!(),
			crate::client::transport::Transport::Tcp(transport) => {
				let path = format!("/packages/{name}/versions/{version}");
				let artifact = transport.get_json(&path).await?;
				Ok(artifact)
			},
		}
	}
}
