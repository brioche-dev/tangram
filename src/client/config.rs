use std::path::PathBuf;
use url::Url;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Config {
	pub transport: Transport,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum Transport {
	#[serde(rename = "in_process")]
	InProcess {
		server: crate::server::config::Config,
	},
	#[serde(rename = "unix")]
	Unix { path: PathBuf },
	#[serde(rename = "tcp")]
	Tcp { url: Url },
}
