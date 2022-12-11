use crate::{util::path_exists, Cli};
use anyhow::Result;
use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Credentials {
	pub email: String,
	pub token: String,
}

impl Cli {
	fn credentials_path() -> Result<PathBuf> {
		Ok(Self::path()?.join("credentials.json"))
	}

	pub async fn read_credentials() -> Result<Option<Credentials>> {
		let path = Self::credentials_path()?;
		if !path_exists(&path).await? {
			return Ok(None);
		}
		let credentials = tokio::fs::read(&path).await?;
		let credentials = serde_json::from_slice(&credentials)?;
		Ok(credentials)
	}

	pub async fn write_credentials(credentials: &Credentials) -> Result<()> {
		let path = Self::credentials_path()?;
		let credentials = serde_json::to_string(credentials)?;
		tokio::fs::write(&path, &credentials).await?;
		Ok(())
	}
}
