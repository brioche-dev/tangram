use std::path::PathBuf;
use url::Url;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Config {
	pub path: PathBuf,
	pub peers: Vec<Url>,
}
