use crate::{rid::Rid, server::Server};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Temp {
	id: Rid,
	path: PathBuf,
	preserve: bool,
}

impl Temp {
	#[must_use]
	pub fn new(server: &Server) -> Temp {
		let id = Rid::gen();
		let path = server.temps_path().join(id.to_string());
		let preserve = server.state.options.preserve_temps;
		Temp { id, path, preserve }
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

impl Drop for Temp {
	fn drop(&mut self) {
		if !self.preserve {
			let path = self.path.clone();
			tokio::task::spawn(async move {
				crate::util::rmrf(&path).await.ok();
			});
		}
	}
}
