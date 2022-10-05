use crate::Cli;
use anyhow::{Context, Result};
use std::path::PathBuf;
use tangram_core::util::path_exists;
use url::Url;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub peers: Option<Vec<Url>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub autoshells: Option<Vec<PathBuf>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_url: Option<Url>,
}

impl Cli {
	fn config_path() -> Result<PathBuf> {
		Ok(Self::path()?.join("config.json"))
	}

	pub async fn read_config() -> Result<Option<Config>> {
		let path = Self::config_path()?;
		let config: Option<Config> = if path_exists(&path).await? {
			let config = tokio::fs::read(&path).await.with_context(|| {
				format!(r#"Failed to read the config from "{}"."#, path.display())
			})?;
			let config = serde_json::from_slice(&config)?;
			Some(config)
		} else {
			None
		};
		Ok(config)
	}

	pub async fn write_config(config: &Config) -> Result<()> {
		let path = Self::config_path()?;
		let bytes = serde_json::to_vec(config)?;
		tokio::fs::write(&path, &bytes)
			.await
			.with_context(|| format!(r#"Failed to write the config to "{}"."#, path.display()))?;
		Ok(())
	}
}
