use crate::{hash::Hash, id, server::Server};
use anyhow::Result;
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
	#[must_use]
	pub fn create_temp(self: &Arc<Self>) -> Temp {
		let id = id::Id::generate();
		let temp_id = Id(id);
		Temp { id: temp_id }
	}

	#[must_use]
	pub fn temps_path(self: &Arc<Self>) -> PathBuf {
		self.path.join("temps")
	}

	#[must_use]
	pub fn temp_path(self: &Arc<Self>, temp: &Temp) -> PathBuf {
		self.path.join("temps").join(temp.id().0.to_string())
	}

	pub async fn checkin_temp(self: &Arc<Self>, temp: Temp) -> Result<Hash> {
		let path = self.temp_path(&temp);
		let artifact = self.checkin(&path).await?;
		Ok(artifact)
	}
}
