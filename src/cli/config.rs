use crate::util::path_exists;
use anyhow::{anyhow, Context, Result};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Config {
	pub client: tangram::client::config::Config,
	pub server: tangram::server::config::Config,
}

mod file {
	use std::path::PathBuf;
	use url::Url;

	#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
	pub struct Config {
		pub client: Option<Client>,
		pub server: Option<Server>,
	}

	#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
	pub struct Client {
		pub transport: Option<tangram::client::config::Transport>,
	}

	#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
	pub struct Server {
		pub path: Option<PathBuf>,
		pub peers: Option<Vec<Url>>,
	}
}

impl Config {
	pub async fn read() -> Result<Config> {
		// Read the config.
		let user_config_path = crate::dirs::home_directory_path()
			.ok_or_else(|| anyhow!("Failed to find the user home directory."))?
			.join(".tangram")
			.join("config.json");
		let user_config: Option<file::Config> = if path_exists(&user_config_path).await? {
			let user_config = tokio::fs::read(&user_config_path).await.with_context(|| {
				format!(
					r#"Failed to read the user configuration from "{}"."#,
					user_config_path.display()
				)
			})?;
			let user_config = serde_json::from_slice(&user_config)?;
			Some(user_config)
		} else {
			None
		};

		// Resolve the transport.
		let user_transport = user_config
			.as_ref()
			.and_then(|config| config.client.as_ref())
			.and_then(|client| client.transport.as_ref())
			.cloned();
		let default_server_path = crate::dirs::home_directory_path()
			.ok_or_else(|| anyhow!("Failed to find the user home directory."))?
			.join(".tangram")
			.join("server");
		let default_transport = tangram::client::config::Transport::InProcess {
			server: tangram::server::config::Config {
				path: default_server_path,
				peers: vec![],
			},
		};
		let transport = user_transport.unwrap_or(default_transport);

		// Resolve the client.
		let client = tangram::client::config::Config { transport };

		// Resolve the path.
		let user_path = user_config
			.as_ref()
			.and_then(|config| config.server.as_ref())
			.and_then(|server| server.path.as_ref())
			.cloned();
		let default_path = crate::dirs::home_directory_path()
			.ok_or_else(|| anyhow!("Failed to find the user home directory."))?
			.join("tangram")
			.join("server");
		let path = user_path.unwrap_or(default_path);

		// Resolve the peers.
		let user_peers = user_config
			.as_ref()
			.and_then(|config| config.server.as_ref())
			.and_then(|server| server.peers.as_ref())
			.cloned();
		let peers = user_peers.unwrap_or_default();

		// Resolve the server.
		let server = tangram::server::config::Config { path, peers };

		// Create the config.
		let config = Config { client, server };

		Ok(config)
	}
}
