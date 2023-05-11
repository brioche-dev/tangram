use crate::{id::Id, instance::Instance, util::fs};

pub struct Temp<'a> {
	_tg: &'a Instance,
	_id: Id,
	path: fs::PathBuf,
}

impl<'a> Temp<'a> {
	pub fn new(tg: &'a Instance) -> Temp<'a> {
		let id = Id::generate();
		let path = tg.temps_path().join(id.to_string());
		Temp {
			_tg: tg,
			_id: id,
			path,
		}
	}

	#[must_use]
	pub fn path(&self) -> &fs::Path {
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
