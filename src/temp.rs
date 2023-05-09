use crate::{id::Id, instance::Instance};
use std::path::{Path, PathBuf};

pub struct Temp<'a> {
	_tg: &'a Instance,
	id: Id,
	path: PathBuf,
}

impl<'a> Temp<'a> {
	pub fn new(tg: &'a Instance) -> Temp<'a> {
		let id = Id::generate();
		let path = tg.temps_path().join(id.to_string());
		Temp { _tg: tg, id, path }
	}

	#[must_use]
	pub fn id(&self) -> Id {
		self.id
	}

	#[must_use]
	pub fn path(&self) -> &Path {
		&self.path
	}
}

impl<'a> Drop for Temp<'a> {
	fn drop(&mut self) {
		let path = self.path.clone();
		tokio::task::spawn(async move {
			crate::util::fs::rmrf(&path).await.ok();
		});
	}
}
