use crate::Cli;
use tangram::{error::Result, util::fs};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Credentials {
	pub email: String,
	pub token: String,
}

impl Cli {
	pub fn credentials_path(&self) -> fs::PathBuf {
		self.tg.path().join("credentials.json")
	}

	pub async fn _read_credentials(&self) -> Result<Option<Credentials>> {
		Self::read_credentials_from_path(&self.credentials_path()).await
	}

	pub async fn read_credentials_from_path(path: &fs::Path) -> Result<Option<Credentials>> {
		let credentials = match tokio::fs::read(&path).await {
			Ok(credentials) => credentials,
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
			Err(error) => return Err(error.into()),
		};
		let credentials = serde_json::from_slice(&credentials)?;
		Ok(credentials)
	}

	pub async fn write_credentials(&self, credentials: &Credentials) -> Result<()> {
		Self::write_credentials_to_path(&self.credentials_path(), credentials).await
	}

	pub async fn write_credentials_to_path(
		path: &fs::Path,
		credentials: &Credentials,
	) -> Result<()> {
		let credentials = serde_json::to_string(credentials)?;
		tokio::fs::write(path, &credentials).await?;
		Ok(())
	}
}
