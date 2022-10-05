use std::path::PathBuf;
use url::Url;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Options {
	pub path: PathBuf,
	pub peers: Vec<Url>,
}
