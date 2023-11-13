use std::path::Path;

use async_trait::async_trait;

use crate::{Client, Result, Artifact, Lock, lock::LockFile};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Package {
	pub name: String,
	pub versions: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub struct Metadata {
	pub name: Option<String>,
	pub version: Option<String>,
	pub description: Option<String>,
}

#[async_trait]
pub trait Builder where Self: Send + Sync {
	fn clone_box(&self) -> Box<dyn Builder>;
	async fn get_package(&self, client: &dyn Client, path: &Path) -> Result<(Artifact, Lock)>;
	async fn update(&mut self, client: &dyn Client, lockfile: Option<LockFile>) -> Result<()>;
}
