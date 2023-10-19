use crate::{util::dirs::user_config_directory_path, Cli, Result};
use std::path::{Path, PathBuf};
use tangram_client::{Wrap, WrapErr};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Credentials {
	/// The user's email.
	pub email: String,

	/// The user's token.
	pub token: String,
}

impl Cli {
	pub fn credentials_path() -> Result<PathBuf> {
		Ok(user_config_directory_path()?
			.join("tangram")
			.join("credentials.json"))
	}

	pub async fn read_credentials() -> Result<Option<Credentials>> {
		Self::read_credentials_from_path(&Self::credentials_path()?).await
	}

	pub async fn read_credentials_from_path(path: &Path) -> Result<Option<Credentials>> {
		let credentials = match tokio::fs::read(&path).await {
			Ok(credentials) => credentials,
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
			Err(error) => return Err(error.wrap("Failed to read the credentials.")),
		};
		let credentials = serde_json::from_slice(&credentials)
			.wrap_err("Failed to deserialize the credentials.")?;
		Ok(credentials)
	}

	pub async fn write_credentials(credentials: &Credentials) -> Result<()> {
		Self::write_credentials_to_path(&Self::credentials_path()?, credentials).await
	}

	pub async fn write_credentials_to_path(path: &Path, credentials: &Credentials) -> Result<()> {
		let credentials =
			serde_json::to_string(credentials).wrap_err("Failed to serialize the credentials.")?;
		tokio::fs::write(path, &credentials)
			.await
			.wrap_err("Failed to write the credentials.")?;
		Ok(())
	}
}
