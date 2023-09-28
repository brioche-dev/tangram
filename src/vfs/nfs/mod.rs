use std::{path::Path, str::Utf8Error};

use crate::vfs::nfs::types::NFS4ERR_IO;

mod compound;
mod ops;
mod rpc;
mod server;
mod state;
mod types;
mod xdr;

pub(crate) use server::Server;

#[derive(Debug)]
pub enum NfsError {
	UnexpectedEof,
	Utf8Error(Utf8Error),
	Io(std::io::Error),
	Custom(String),
}

impl From<Utf8Error> for NfsError {
	fn from(value: Utf8Error) -> Self {
		Self::Utf8Error(value)
	}
}

impl From<std::io::Error> for NfsError {
	fn from(value: std::io::Error) -> Self {
		Self::Io(value)
	}
}

impl From<NfsError> for i32 {
	fn from(value: NfsError) -> Self {
		tracing::error!(?value);
		NFS4ERR_IO
	}
}

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
		.ok_or(crate::error::error!("Failed to mount NFS share."))
}
