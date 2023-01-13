use crate::{util::path_exists, Cli};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use url::Url;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub autoshells: Option<Vec<PathBuf>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_url: Option<Url>,
}

impl Cli {
	pub async fn read_config(&self) -> Result<Option<Config>> {
		Self::read_config_from_path(&self.config_path()).await
	}

	pub async fn read_config_from_path(path: &Path) -> Result<Option<Config>> {
		let config: Option<Config> = if path_exists(path).await? {
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

	pub async fn write_config(&self, config: &Config) -> Result<()> {
		Self::write_config_to_path(&self.config_path(), config).await
	}

	pub async fn write_config_to_path(path: &Path, config: &Config) -> Result<()> {
		let bytes = serde_json::to_vec(config)?;
		tokio::fs::write(&path, &bytes)
			.await
			.with_context(|| format!(r#"Failed to write the config to "{}"."#, path.display()))?;
		Ok(())
	}
}
