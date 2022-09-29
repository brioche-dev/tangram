use anyhow::Result;
use std::path::{Path, PathBuf};
use url::Url;

#[derive(Clone, Debug)]
pub struct Config {
	pub peers: Vec<Url>,
	pub autoshells: Vec<PathBuf>,
}

pub mod file {
	use anyhow::{Context, Result};
	use std::path::{Path, PathBuf};
	use tangram::util::path_exists;
	use url::Url;

	#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
	pub struct Config {
		#[serde(skip_serializing_if = "Option::is_none")]
		pub peers: Option<Vec<Url>>,
		#[serde(skip_serializing_if = "Option::is_none")]
		pub autoshells: Option<Vec<PathBuf>>,
	}

	impl Config {
		pub async fn read(path: &Path) -> Result<Option<Config>> {
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

		pub async fn write(&self, path: &Path) -> Result<()> {
			let bytes = serde_json::to_vec(self)?;
			tokio::fs::write(&path, &bytes).await.with_context(|| {
				format!(r#"Failed to write the config to "{}"."#, path.display())
			})?;
			Ok(())
		}
	}
}

impl Config {
	pub async fn read(path: &Path) -> Result<Config> {
		let config = file::Config::read(path).await?;

		// Resolve the peers.
		let peers = config
			.as_ref()
			.and_then(|config| config.peers.as_ref())
			.cloned();
		let peers = peers.unwrap_or_default();

		// Resolve the autoshells.
		let autoshells = config
			.as_ref()
			.and_then(|config| config.autoshells.as_ref())
			.cloned();
		let autoshells = autoshells.unwrap_or_default();

		// Create the config.
		let config = Config { peers, autoshells };

		Ok(config)
	}
}
