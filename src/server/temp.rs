use crate::{hash::Hash, id, server::Server};
use anyhow::{anyhow, Result};
use camino::Utf8Path;
use std::{path::PathBuf, sync::Arc};

#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub id::Id);

#[derive(Clone)]
pub struct Temp {
	id: Id,
}

impl Temp {
	#[must_use]
	pub fn id(&self) -> Id {
		self.id
	}
}

impl Server {
	pub async fn create_temp(self: &Arc<Self>) -> Result<Temp> {
		let id = id::Id::generate();
		let temp_id = Id(id);
		let temp = Temp { id: temp_id };
		Ok(temp)
	}

	#[must_use]
	pub fn temps_path(self: &Arc<Self>) -> PathBuf {
		self.path.join("temps")
	}

	#[must_use]
	pub fn temp_path(self: &Arc<Self>, temp: &Temp) -> PathBuf {
		self.path.join("temps").join(temp.id().0.to_string())
	}

	pub async fn temp_add_dependency(
		self: &Arc<Self>,
		temp: &mut Temp,
		path: &Utf8Path,
		artifact: Hash,
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

	pub async fn checkin_temp(self: &Arc<Self>, temp: Temp) -> Result<Hash> {
		let path = self.temp_path(&temp);
		let artifact = self.checkin(&path).await?;
		Ok(artifact)
	}
}
