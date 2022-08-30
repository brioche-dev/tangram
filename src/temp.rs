use crate::{id::Id, server::Server};
use derive_more::Deref;
use std::sync::Arc;

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
