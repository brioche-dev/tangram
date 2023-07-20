use crate::{instance::Instance, rid::Rid};
use std::path::{Path, PathBuf};

pub struct Temp<'a> {
	tg: &'a Instance,
	id: Rid,
	path: PathBuf,
}

impl<'a> Temp<'a> {
	#[must_use]
	pub fn new(tg: &Instance) -> Temp {
		let id = Rid::gen();
		let path = tg.temps_path().join(id.to_string());
		Temp { tg, id, path }
	}

	#[must_use]
	pub fn id(&self) -> Rid {
		self.id
	}

	#[must_use]
	pub fn path(&self) -> &Path {
		&self.path
	}
}

impl<'a> Drop for Temp<'a> {
	fn drop(&mut self) {
		if !self.tg.options.preserve_temps {
			let path = self.path.clone();
			tokio::task::spawn(async move {
				crate::util::fs::rmrf(&path).await.ok();
			});
		}
	}
}
