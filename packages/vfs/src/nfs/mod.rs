use std::path::Path;
use tangram_client as tg;
use tg::WrapErr;

mod compound;
mod ops;
mod rpc;
mod server;
mod state;
mod types;
mod xdr;

pub use server::Server;

pub async fn mount(mountpoint: &Path, port: u16) -> crate::Result<()> {
	let _ = tokio::process::Command::new("umount")
		.arg("-f")
		.arg(mountpoint)
		.status()
		.await
		.wrap_err("Failed to unmount.")?;
	tokio::process::Command::new("mount_nfs")
		.arg("-o")
		.arg(format!("tcp,vers=4.0,port={port}"))
		.arg("localhost:")
		.arg(mountpoint)
		.status()
		.await
		.wrap_err("Failed to mount.")?
		.success()
		.then_some(())
		.wrap_err("Failed to mount NFS share.")?;
	Ok(())
}
