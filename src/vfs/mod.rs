use crate::Result;
use std::path::PathBuf;

#[cfg(target_os = "linux")]
mod fuse;
mod nfs;

pub enum Server {
	// Nfs(nfs::Server),
	// #[cfg(target_os = "linux")]
	// Fuse(fuse::Server),
}

impl Server {
	pub fn new(path: PathBuf, client: crate::Client) -> Server {
		todo!()
	}

	pub async fn serve(&self) -> Result<()> {
		todo!()
	}
}
