use crate::Result;
use derive_more::FromStr;
use std::path::PathBuf;
use tokio::task::JoinHandle;

mod fuse;
mod nfs;

pub enum Server {
	Fuse(fuse::Server),
	Nfs(nfs::Server, u16),
}

#[derive(Copy, Clone, Debug)]
pub enum Kind {
	Fuse,
	Nfs(u16),
}

impl FromStr for Kind {
	type Err = &'static str;
	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		match s {
			"fuse" => Ok(Self::Fuse),
			"nfs" => Ok(Self::Nfs(2049)),
			_ => Err(r#"Expected "fuse" or "nfs"."#),
		}
	}
}

impl Server {
	#[must_use]
	pub fn new(kind: Kind, client: crate::Client) -> Server {
		match kind {
			Kind::Fuse => Server::Fuse(fuse::Server::new(client)),
			Kind::Nfs(port) => Server::Nfs(nfs::Server::new(client), port),
		}
	}

	pub async fn mount(self, path: PathBuf) -> Result<JoinHandle<Result<()>>> {
		tracing::info!("Mounting tgvfs at {path:#?}.");
		match self {
			Server::Fuse(server) => {
				let fuse_file = fuse::mount(path).await?;
				let task = tokio::task::spawn_blocking(move || {
					let error = server.serve(fuse_file);
					tracing::error!(?error, "Whoops.");
					error
				});
				Ok(task)
			},
			Server::Nfs(server, port) => {
				// Spawn the server task.
				let task = tokio::spawn(async move {
					let error = server.serve(port).await;
					tracing::error!(?error, "Whoops.");
					error
				});
				nfs::mount(&path, port).await?;
				Ok(task)
			},
		}
	}
}
