use crate::Cli;
use tangram::{
	error::{Context, Result},
	util::fs,
};
use url::Url;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub autoshells: Option<Vec<fs::PathBuf>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_url: Option<Url>,
}

impl Cli {
	pub fn config_path(&self) -> fs::PathBuf {
		self.tg.path().join("config.json")
	}

	pub async fn read_config(&self) -> Result<Option<Config>> {
		Self::read_config_from_path(&self.config_path()).await
	}

	pub async fn read_config_from_path(path: &fs::Path) -> Result<Option<Config>> {
		let config = match tokio::fs::read(&path).await {
			Ok(config) => config,
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
			Err(error) => return Err(error.into()),
		};
		let config =
			serde_json::from_slice(&config).context("Failed to deserialize the config.")?;
		Ok(config)
	}

	pub async fn write_config(&self, config: &Config) -> Result<()> {
		Self::write_config_to_path(&self.config_path(), config).await
	}

	pub async fn write_config_to_path(path: &fs::Path, config: &Config) -> Result<()> {
		let bytes = serde_json::to_vec(config)?;
		tokio::fs::write(&path, &bytes).await.with_context(|| {
			let path = path.display();
			format!(r#"Failed to write the config to "{path}"."#)
		})?;
		Ok(())
	}
}
