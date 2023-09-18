use crate::{
	error::{Error, Result, WrapErr},
	Cli,
};
use std::path::{Path, PathBuf};
use tg::util::dirs::user_config_directory_path;
use url::Url;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub autoenvs: Option<Vec<PathBuf>>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub parent_url: Option<Url>,
}

impl Cli {
	pub fn config_path() -> Result<PathBuf> {
		Ok(user_config_directory_path()?
			.join("tangram")
			.join("config.json"))
	}

	#[tracing::instrument(level = "debug", skip_all)]
	pub async fn read_config() -> Result<Option<Config>> {
		Self::read_config_from_path(&Self::config_path()?).await
	}

	#[tracing::instrument(level = "debug")]
	pub async fn read_config_from_path(path: &Path) -> Result<Option<Config>> {
		let config = match tokio::fs::read(&path).await {
			Ok(config) => config,
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
			Err(error) => return Err(Error::other(error)),
		};
		let config = serde_json::from_slice(&config)
			.map_err(Error::other)
			.wrap_err("Failed to deserialize the config.")?;
		Ok(config)
	}

	pub async fn write_config(config: &Config) -> Result<()> {
		Self::write_config_to_path(&Self::config_path()?, config).await
	}

	pub async fn write_config_to_path(path: &Path, config: &Config) -> Result<()> {
		let bytes = serde_json::to_vec(config).map_err(Error::other)?;
		tokio::fs::write(&path, &bytes)
			.await
			.map_err(Error::other)
			.wrap_err_with(|| {
				let path = path.display();
				format!(r#"Failed to write the config to "{path}"."#)
			})?;
		Ok(())
	}
}
