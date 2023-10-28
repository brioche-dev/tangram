use std::path::Path;
use tangram_client as tg;
use tg::Result;

#[cfg(target_os = "linux")]
mod fuse;
#[cfg(target_os = "macos")]
mod nfs;

#[cfg(target_os = "macos")]
const PORT: u16 = 8437;

pub enum Server {
	#[cfg(target_os = "linux")]
	Fuse(fuse::Server),
	#[cfg(target_os = "macos")]
	Nfs(nfs::Server),
}

impl Server {
	#[must_use]
	pub fn new(client: &dyn tg::Client) -> Self {
		#[cfg(target_os = "linux")]
		{
			Self::Fuse(fuse::Server::new(client))
		}
		#[cfg(target_os = "macos")]
		{
			Self::Nfs(nfs::Server::new(client))
		}
	}

	pub async fn mount(self, path: &Path) -> Result<tokio::task::JoinHandle<Result<()>>> {
		match self {
			#[cfg(target_os = "linux")]
			Server::Fuse(server) => {
				let file = fuse::mount(path).await?;
				let task = tokio::task::spawn(async move { server.serve(file).await });
				Ok(task)
			},

			#[cfg(target_os = "macos")]
			Server::Nfs(server) => {
				let task = tokio::spawn(async move { server.serve(PORT).await });
				nfs::mount(path, PORT).await?;
				Ok(task)
			},
		}
	}
}
