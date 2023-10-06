use crate::error;
use std::path::Path;

mod compound;
mod ops;
mod rpc;
mod server;
mod state;
mod types;
mod xdr;

pub(crate) use server::Server;

pub async fn mount(mountpoint: &Path, port: u16) -> crate::Result<()> {
	let _ = tokio::process::Command::new("umount")
		.arg("-f")
		.arg(mountpoint)
		.status()
		.await?;
	tokio::process::Command::new("mount_nfs")
		.arg("-o")
		.arg(format!("tcp,vers=4.0,port={port}"))
		.arg("localhost:")
		.arg(mountpoint)
		.status()
		.await?
		.success()
		.then_some(())
		.ok_or(error!("Failed to mount NFS share."))
}
