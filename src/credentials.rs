use crate::{util::path_exists, Cli};
use anyhow::Result;
use std::path::Path;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Credentials {
	pub email: String,
	pub token: String,
}

impl Cli {
	pub async fn read_credentials(&self) -> Result<Option<Credentials>> {
		Self::read_credentials_from_path(&self.credentials_path()).await
	}

	pub async fn read_credentials_from_path(path: &Path) -> Result<Option<Credentials>> {
		if !path_exists(path).await? {
			return Ok(None);
		}
		let credentials = tokio::fs::read(&path).await?;
		let credentials = serde_json::from_slice(&credentials)?;
		Ok(credentials)
	}

	pub async fn write_credentials(&self, credentials: &Credentials) -> Result<()> {
		Self::write_credentials_to_path(&self.credentials_path(), credentials).await
	}

	pub async fn write_credentials_to_path(path: &Path, credentials: &Credentials) -> Result<()> {
		let credentials = serde_json::to_string(credentials)?;
		tokio::fs::write(path, &credentials).await?;
		Ok(())
	}
}
