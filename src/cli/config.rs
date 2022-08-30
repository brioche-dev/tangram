use crate::dirs::{global_config_directory_path, home_directory_path, user_config_directory_path};
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use url::Url;

async fn path_exists(path: &Path) -> Result<bool> {
	match tokio::fs::metadata(&path).await {
		Ok(_) => Ok(true),
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
		Err(error) => Err(error.into()),
	}
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Config {
	pub transport: Transport,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum Transport {
	#[serde(rename = "in_process")]
	InProcess { path: PathBuf },
	#[serde(rename = "unix")]
	Unix { path: PathBuf },
	#[serde(rename = "tcp")]
	Tcp { url: Url },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Partial {
	transport: Option<Transport>,
}

impl Config {
	pub async fn read() -> Result<Config> {
		// Read the global config.
		let global_config_path = global_config_directory_path()
			.ok_or_else(|| anyhow!("Unable to find the global config directory."))?
			.join("tangram")
			.join("config.json");
		let global_config: Option<Partial> = if path_exists(&global_config_path).await? {
			let global_config = tokio::fs::read(&global_config_path)
				.await
				.with_context(|| {
					anyhow!(
						r#"Failed to read the global configuration from "{}"."#,
						global_config_path.display()
					)
				})?;
			let global_config = serde_json::from_slice(&global_config)?;
			Some(global_config)
		} else {
			None
		};

		// Read the user config.
		let user_config_path = user_config_directory_path()
			.ok_or_else(|| anyhow!("Unable to find the user config directory."))?
			.join("tangram")
			.join("config.json");
		let user_config: Option<Partial> = if path_exists(&user_config_path).await? {
			let user_config = tokio::fs::read(&user_config_path).await.with_context(|| {
				anyhow!(
					r#"Failed to read the user configuration from "{}"."#,
					user_config_path.display()
				)
			})?;
			let user_config = serde_json::from_slice(&user_config)?;
			Some(user_config)
		} else {
			None
		};

		// Get the transport.
		let global_transport = global_config
			.as_ref()
			.and_then(|config| config.transport.clone());
		let user_transport = user_config
			.as_ref()
			.and_then(|config| config.transport.clone());
		let transport = if let Some(transport) = user_transport.or(global_transport) {
			transport
		} else {
			let default_server_path = home_directory_path()
				.ok_or_else(|| anyhow!("Failed to find home directory."))?
				.join(".tangram");
			Transport::InProcess {
				path: default_server_path,
			}
		};

		// Create the config.
		let config = Config { transport };

		Ok(config)
	}
}
