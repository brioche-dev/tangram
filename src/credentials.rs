use crate::{os, Cli};
use anyhow::Result;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Credentials {
	pub email: String,
	pub token: String,
}

impl Cli {
	pub async fn read_credentials(&self) -> Result<Option<Credentials>> {
		Self::read_credentials_from_path(&self.credentials_path()).await
	}

	pub async fn read_credentials_from_path(path: &os::Path) -> Result<Option<Credentials>> {
		if !os::fs::exists(path).await? {
			return Ok(None);
		}
		let credentials = tokio::fs::read(&path).await?;
		let credentials = serde_json::from_slice(&credentials)?;
		Ok(credentials)
	}

	pub async fn write_credentials(&self, credentials: &Credentials) -> Result<()> {
		Self::write_credentials_to_path(&self.credentials_path(), credentials).await
	}

	pub async fn write_credentials_to_path(
		path: &os::Path,
		credentials: &Credentials,
	) -> Result<()> {
		let credentials = serde_json::to_string(credentials)?;
		tokio::fs::write(path, &credentials).await?;
		Ok(())
	}
}
