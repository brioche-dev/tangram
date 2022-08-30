use crate::{
	artifact::Artifact,
	id::Id,
	server::Server,
	temp::{Temp, TempId},
};
use anyhow::{anyhow, Result};
use camino::Utf8Path;
use std::{path::PathBuf, sync::Arc};

impl Server {
	pub async fn create_temp(self: &Arc<Self>) -> Result<Temp> {
		let id = Id::generate();
		let temp_id = TempId(id);
		let temp = Temp {
			id: temp_id,
			server: Arc::clone(self),
		};
		self.temps.lock().await.insert(temp_id, temp.clone());
		Ok(temp)
	}

	#[must_use]
	pub fn temp_path(self: &Arc<Self>, temp: &Temp) -> PathBuf {
		self.path.join("temps").join(temp.id().to_string())
	}

	pub async fn add_dependency(
		self: &Arc<Self>,
		temp: &mut Temp,
		path: &Utf8Path,
		artifact: Artifact,
	) -> Result<()> {
		// Create a fragment for the dependency.
		let dependency_fragment = self.create_fragment(&artifact).await?;

		// Create a symlink from `path` within `temp` to `dependency.path` within the `dependency_fragment`.
		let symlink_path = self.temp_path(temp).join(path);
		let symlink_target = dependency_fragment.path();
		let symlink_parent_path = symlink_path
			.parent()
			.ok_or_else(|| anyhow!("Failed to get the parent for the symlink path."))?;
		tokio::fs::create_dir_all(&symlink_parent_path).await?;
		tokio::fs::symlink(&symlink_target, &symlink_path).await?;

		Ok(())
	}

	pub async fn checkin_temp(self: &Arc<Self>, temp: Temp) -> Result<Artifact> {
		let path = self.temp_path(&temp);
		let artifact = self.checkin(&path).await?;
		Ok(artifact)
	}

	pub async fn drop_temp(self: &Arc<Self>, temp_id: TempId) -> Result<()> {
		self.temps.lock().await.remove(&temp_id);
		Ok(())
	}
}
