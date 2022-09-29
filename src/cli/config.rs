use std::path::PathBuf;

use crate::util::path_exists;
use anyhow::{anyhow, Context, Result};
use url::Url;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Config {
	pub path: PathBuf,
	pub peers: Vec<Url>,
}

mod file {
	use std::path::PathBuf;
	use url::Url;

	#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
	pub struct Config {
		pub path: Option<PathBuf>,
		pub peers: Option<Vec<Url>>,
	}
}

impl Config {
	pub async fn read() -> Result<Config> {
		// Read the config.
		let config_path = crate::dirs::home_directory_path()
			.ok_or_else(|| anyhow!("Failed to find the user home directory."))?
			.join(".tangram")
			.join("config.json");
		let config: Option<file::Config> = if path_exists(&config_path).await? {
			let user_config = tokio::fs::read(&config_path).await.with_context(|| {
				format!(
					r#"Failed to read the user configuration from "{}"."#,
					config_path.display()
				)
			})?;
			let user_config = serde_json::from_slice(&user_config)?;
			Some(user_config)
		} else {
			None
		};

		// Resolve the path.
		let path = config
			.as_ref()
			.and_then(|config| config.path.as_ref())
			.cloned();
		let default_path = crate::dirs::home_directory_path()
			.ok_or_else(|| anyhow!("Failed to find the user home directory."))?
			.join("tangram")
			.join("server");
		let path = path.unwrap_or(default_path);

		// Resolve the peers.
		let peers = config
			.as_ref()
			.and_then(|config| config.peers.as_ref())
			.cloned();
		let peers = peers.unwrap_or_default();

		// Create the config.
		let config = Config { path, peers };

		Ok(config)
	}
}
