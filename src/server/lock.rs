use anyhow::Result;
use std::{
	path::PathBuf,
	sync::{Arc, Weak},
};
use tokio::sync::RwLock;

pub struct Lock {
	path: PathBuf,
	shared: RwLock<Option<Weak<tokio::fs::File>>>,
}

pub struct SharedGuard {
	_file: Arc<tokio::fs::File>,
}

pub struct ExclusiveGuard {
	_file: tokio::fs::File,
}

impl Lock {
	pub fn new(path: PathBuf) -> Lock {
		let shared = RwLock::new(None);
		Lock { path, shared }
	}

	pub async fn lock_shared(&self) -> Result<SharedGuard> {
		let file = { self.shared.read().await.as_ref().and_then(Weak::upgrade) };
		let file = if let Some(file) = file {
			file
		} else {
			let file = tokio::fs::OpenOptions::new()
				.read(true)
				.write(true)
				.create(true)
				.open(&self.path)
				.await?;
			self::sys::lock_shared(&file).await?;
			let file = Arc::new(file);
			self.shared.write().await.replace(Arc::downgrade(&file));
			file
		};
		Ok(SharedGuard { _file: file })
	}

	pub async fn lock_exclusive(&self) -> Result<ExclusiveGuard> {
		let file = tokio::fs::OpenOptions::new()
			.read(true)
			.write(true)
			.create(true)
			.open(&self.path)
			.await?;
		self::sys::lock_exclusive(&file).await?;
		Ok(ExclusiveGuard { _file: file })
	}
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod sys {
	use anyhow::{anyhow, bail, Result};
	use libc::{flock, LOCK_EX, LOCK_SH, LOCK_UN};
	use std::os::unix::io::AsRawFd;

	pub(super) async fn lock_shared(file: &tokio::fs::File) -> Result<()> {
		let fd = file.as_raw_fd();
		let ret = tokio::task::spawn_blocking(move || unsafe { flock(fd, LOCK_SH) })
			.await
			.unwrap();
		if ret != 0 {
			bail!(anyhow!(std::io::Error::last_os_error()).context("The flock syscall failed."));
		}
		Ok(())
	}

	pub(super) async fn lock_exclusive(file: &tokio::fs::File) -> Result<()> {
		let fd = file.as_raw_fd();
		let ret = tokio::task::spawn_blocking(move || unsafe { flock(fd, LOCK_EX) })
			.await
			.unwrap();
		if ret != 0 {
			bail!(anyhow!(std::io::Error::last_os_error()).context("The flock syscall failed."));
		}
		Ok(())
	}

	pub fn _unlock(file: &tokio::fs::File) -> Result<()> {
		let fd = file.as_raw_fd();
		let ret = unsafe { flock(fd, LOCK_UN) };
		if ret != 0 {
			bail!(anyhow!(std::io::Error::last_os_error()).context("The flock syscall failed."));
		}
		Ok(())
	}
}
