use crate::{artifact::Artifact, client::Client, manifest::Manifest};
use anyhow::{Context, Result};
use std::path::Path;

impl Client {
	pub async fn get_package(&self, name: &str, version: &str) -> Result<Option<Artifact>> {
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

	pub async fn publish_package(&self, package_path: &Path) -> Result<Artifact> {
		// TODO.
		let locked = false;

		// Checkin the package.
		let package = self
			.checkin_package(package_path, locked)
			.await
			.context("Failed to check in package")?;

		// Read the manifest.
		let manifest_path = package_path.join("tangram.json");
		let manifest = tokio::fs::read(&manifest_path)
			.await
			.context("Failed to read the package manifest.")?;
		let manifest: Manifest = serde_json::from_slice(&manifest).with_context(|| {
			format!(
				r#"Failed to parse the package manifest at path "{}"."#,
				manifest_path.display()
			)
		})?;

		let name = manifest.name;
		let version = manifest.version;
		let artifact = package;

		match &self.transport {
			crate::client::transport::Transport::InProcess(server) => {
				let artifact = server
					.create_package_version(&name, &version, artifact)
					.await?;
				Ok(artifact)
			},
			crate::client::transport::Transport::Unix(_) => todo!(),
			crate::client::transport::Transport::Tcp(transport) => {
				// Create the package version.
				// TODO this could error if there is already a package with that name and version combo.
				let path = format!("/packages/{name}/versions/{version}");
				let artifact: Artifact = transport.post_json(&path, &artifact).await?;
				Ok(artifact)
			},
		}
	}
}
