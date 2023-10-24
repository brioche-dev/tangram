use std::path::PathBuf;
use tangram_client as tg;
use tg::Result;

#[cfg(target_os = "linux")]
mod fuse;
// #[cfg(target_os = "macos")]
// mod nfs;

pub enum Server {
	#[cfg(target_os = "linux")]
	Fuse(fuse::Server),
	// #[cfg(target_os = "macos")]
	// Nfs(nfs::Server, u16),
}

impl Server {
	#[must_use]
	pub fn new(_client: &dyn tg::Client) -> Self {
		#[cfg(target_os = "linux")]
		{
			Self::Fuse(fuse::Server::new(client))
		}
		#[cfg(target_os = "macos")]
		{
			// Self::Nfs(nfs::Server::new(client), port)
			todo!()
		}
	}

	pub async fn mount(self, _path: PathBuf) -> Result<tokio::task::JoinHandle<Result<()>>> {
		todo!()
		// match self {
		// 	#[cfg(target_os = "linux")]
		// 	Server::Fuse(server) => {
		// 		let file = fuse::mount(path).await?;
		// 		let task = tokio::task::spawn_blocking(move || server.serve(file));
		// 		Ok(task)
		// 	},

		// 	#[cfg(target_os = "macos")]
		// 	Server::Nfs(server, port) => {
		// 		let task = tokio::spawn(async move { server.serve(port).await });
		// 		nfs::mount(&path, port).await?;
		// 		Ok(task)
		// 	},
		// }
	}
}
