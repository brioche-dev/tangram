use crate::{error::Error, Cli, Result};
use tangram::util::{dirs::user_config_directory_path, fs};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Credentials {
	pub email: String,
	pub token: String,
}

impl Cli {
	pub fn credentials_path() -> Result<fs::PathBuf> {
		Ok(user_config_directory_path()?
			.join("tangram")
			.join("credentials.json"))
	}

	pub async fn read_credentials() -> Result<Option<Credentials>> {
		Self::read_credentials_from_path(&Self::credentials_path()?).await
	}

	pub async fn read_credentials_from_path(path: &fs::Path) -> Result<Option<Credentials>> {
		let credentials = match tokio::fs::read(&path).await {
			Ok(credentials) => credentials,
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
			Err(error) => return Err(Error::other(error)),
		};
		let credentials = serde_json::from_slice(&credentials).map_err(Error::other)?;
		Ok(credentials)
	}

	pub async fn write_credentials(credentials: &Credentials) -> Result<()> {
		Self::write_credentials_to_path(&Self::credentials_path()?, credentials).await
	}

	pub async fn write_credentials_to_path(
		path: &fs::Path,
		credentials: &Credentials,
	) -> Result<()> {
		let credentials = serde_json::to_string(credentials).map_err(Error::other)?;
		tokio::fs::write(path, &credentials)
			.await
			.map_err(Error::other)?;
		Ok(())
	}
}
