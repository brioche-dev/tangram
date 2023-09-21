use crate::server::Server;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Temp {
	path: PathBuf,
}

impl Temp {
	#[must_use]
	pub fn new(server: &Server) -> Temp {
		let id: [u8; 16] = rand::random();
		let path = server.temps_path().join(hex::encode(id));
		Temp { path }
	}

	#[must_use]
	pub fn path(&self) -> &Path {
		&self.path
	}
}

impl Drop for Temp {
	fn drop(&mut self) {
		let path = self.path.clone();
		tokio::task::spawn(async move {
			crate::util::rmrf(&path).await.ok();
		});
	}
}
