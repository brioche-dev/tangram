use crate::{artifact::Artifact, id::Id, server::Server};
use anyhow::{anyhow, Result};
use camino::Utf8Path;
use derive_more::Deref;
use std::{path::PathBuf, sync::Arc};

#[allow(clippy::module_name_repetitions)]
#[derive(Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TempId(pub Id);

#[derive(Clone)]
pub struct Temp {
	pub(crate) server: Arc<Server>,
	pub(crate) id: TempId,
}

impl Temp {
	#[must_use]
	pub fn id(&self) -> TempId {
		self.id
	}
}

impl Drop for Temp {
	fn drop(&mut self) {
		let server = Arc::clone(&self.server);
		let id = self.id();
		tokio::spawn(async move { server.drop_temp(id).await.ok() });
	}
}

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
		let dependency_fragment = self.create_fragment(artifact).await?;

		// Create a symlink from `path` within `temp` to `dependency.path` within the `dependency_fragment`.
		let symlink_path = self.temp_path(temp).join(path);
		let symlink_target = self.fragment_path(&dependency_fragment);
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
