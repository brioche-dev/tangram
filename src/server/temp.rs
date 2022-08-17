use crate::{artifact::Artifact, client::Client, id::Id, server::Server};
use anyhow::Result;
use derive_more::Deref;
use std::{path::PathBuf, sync::Arc};

#[allow(clippy::module_name_repetitions)]
#[derive(Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TempId(pub Id);

#[derive(Clone)]
pub struct Temp {
	pub client: Option<Arc<Client>>,
	pub id: TempId,
}

impl Server {
	pub async fn create_temp(self: &Arc<Self>) -> Result<Temp> {
		let id = Id::generate();
		let temp_id = TempId(id);
		let temp = Temp {
			client: Some(Arc::new(Client::new_in_process(Arc::clone(self)))),
			id: temp_id,
		};
		self.temps.lock().await.insert(temp_id, temp.clone());
		Ok(temp)
	}

	#[must_use]
	pub fn temp_path(self: &Arc<Self>, temp: &Temp) -> PathBuf {
		self.path.join("temps").join(temp.id.to_string())
	}

	// pub(super) fn add_dependency(
	// 	self: &Arc<Self>,
	// 	temp: &Temp,
	// 	path: Utf8PathBuf,
	// 	dependency: Dependency,
	// ) -> Result<()> {
	// 	// TODO Create a symlink at `path` that points to `dependency` checked out to a fragment and set the appropriate xattr.
	// 	todo!()
	// }

	pub async fn checkin_temp(self: &Arc<Self>, temp: Temp) -> Result<Artifact> {
		let path = self.temp_path(&temp);
		let client = Client::new_in_process(Arc::clone(self));
		let artifact = client.checkin(&path).await?;
		Ok(artifact)
	}
}

// impl Drop for Temp {
// 	fn drop(&mut self) {
// 		let client = self.client.take().unwrap();
// 		let id = self.id;
// 		tokio::task::spawn(client.remove_temp(id).ok());
// 	}
// }
