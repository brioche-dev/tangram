use crate::{error::Result, util::fs};
use std::{
	cell::UnsafeCell,
	sync::{Arc, Weak},
};
use tokio::sync::RwLock;

pub struct Lock<T> {
	pub path: fs::PathBuf,
	pub value: UnsafeCell<T>,
	pub shared_lock_file: Arc<RwLock<Option<Weak<tokio::fs::File>>>>,
}

unsafe impl<T> Send for Lock<T> where T: Send {}
unsafe impl<T> Sync for Lock<T> where T: Send {}

pub struct SharedGuard<'a, T> {
	pub value: &'a T,
	pub lock_file: Arc<tokio::fs::File>,
}

pub struct ExclusiveGuard<'a, T> {
	pub value: &'a mut T,
	pub lock_file: Arc<tokio::fs::File>,
}

impl<T> Lock<T> {
	pub fn new(path: &fs::Path, value: T) -> Lock<T> {
		let shared_lock_file = Arc::new(RwLock::new(None));
		Lock {
			path: path.to_owned(),
			value: UnsafeCell::new(value),
			shared_lock_file,
		}
	}

	pub async fn try_lock_shared(&self) -> Result<Option<SharedGuard<'_, T>>> {
		let lock_file = {
			self.shared_lock_file
				.read()
				.await
				.as_ref()
				.and_then(Weak::upgrade)
		};
		if let Some(lock_file) = lock_file {
			return Ok(Some(SharedGuard {
				value: unsafe { &*self.value.get() },
				lock_file,
			}));
		}
		let lock_file = tokio::fs::OpenOptions::new()
			.read(true)
			.write(true)
			.create(true)
			.open(&self.path)
			.await?;
		let locked = self::sys::try_lock_shared(&lock_file)?;
		if locked {
			let lock_file = Arc::new(lock_file);
			self.shared_lock_file
				.write()
				.await
				.replace(Arc::downgrade(&lock_file));
			Ok(Some(SharedGuard {
				value: unsafe { &*self.value.get() },
				lock_file,
			}))
		} else {
			Ok(None)
		}
	}

	pub async fn try_lock_exclusive(&self) -> Result<Option<ExclusiveGuard<'_, T>>> {
		let lock_file = tokio::fs::OpenOptions::new()
			.read(true)
			.write(true)
			.create(true)
			.open(&self.path)
			.await?;
		let locked = self::sys::try_lock_exclusive(&lock_file)?;
		if locked {
			let lock_file = Arc::new(lock_file);
			Ok(Some(ExclusiveGuard {
				value: unsafe { &mut *self.value.get() },
				lock_file,
			}))
		} else {
			Ok(None)
		}
	}

	pub async fn lock_shared(&self) -> Result<SharedGuard<'_, T>> {
		let lock_file = {
			self.shared_lock_file
				.read()
				.await
				.as_ref()
				.and_then(Weak::upgrade)
		};
		let lock_file = if let Some(lock_file) = lock_file {
			lock_file
		} else {
			let lock_file = tokio::fs::OpenOptions::new()
				.read(true)
				.write(true)
				.create(true)
				.open(&self.path)
				.await?;
			self::sys::lock_shared(&lock_file).await?;
			let lock_file = Arc::new(lock_file);
			self.shared_lock_file
				.write()
				.await
				.replace(Arc::downgrade(&lock_file));
			lock_file
		};
		Ok(SharedGuard {
			value: unsafe { &*self.value.get() },
			lock_file,
		})
	}

	pub async fn lock_exclusive(&self) -> Result<ExclusiveGuard<'_, T>> {
		let lock_file = tokio::fs::OpenOptions::new()
			.read(true)
			.write(true)
			.create(true)
			.open(&self.path)
			.await?;
		self::sys::lock_exclusive(&lock_file).await?;
		let lock_file = Arc::new(lock_file);
		Ok(ExclusiveGuard {
			value: unsafe { &mut *self.value.get() },
			lock_file,
		})
	}
}

impl<'a, T> std::ops::Deref for SharedGuard<'a, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.value
	}
}

impl<'a, T> std::ops::Deref for ExclusiveGuard<'a, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.value
	}
}

impl<'a, T> std::ops::DerefMut for ExclusiveGuard<'a, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.value
	}
}

impl<'a, T> ExclusiveGuard<'a, T> {
	#[must_use]
	pub fn as_shared(&self) -> SharedGuard<T> {
		SharedGuard {
			value: self.value,
			lock_file: self.lock_file.clone(),
		}
	}
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod sys {
	use crate::{
		error::{bail, Error, Result},
		util::errno::errno,
	};
	use libc::{flock, EWOULDBLOCK, LOCK_EX, LOCK_NB, LOCK_SH, LOCK_UN};
	use std::os::unix::io::AsRawFd;

	pub fn try_lock_shared(file: &tokio::fs::File) -> Result<bool> {
		let fd = file.as_raw_fd();
		let ret = unsafe { flock(fd, LOCK_SH | LOCK_NB) };
		match ret {
			0 => Ok(true),
			_ if errno() == EWOULDBLOCK => Ok(false),
			_ => {
				bail!(Error::from(std::io::Error::last_os_error())
					.context("The flock syscall failed."))
			},
		}
	}

	pub fn try_lock_exclusive(file: &tokio::fs::File) -> Result<bool> {
		let fd = file.as_raw_fd();
		let ret = unsafe { flock(fd, LOCK_EX | LOCK_NB) };
		match ret {
			0 => Ok(true),
			_ if errno() == EWOULDBLOCK => Ok(false),
			_ => {
				bail!(Error::from(std::io::Error::last_os_error())
					.context("The flock syscall failed."))
			},
		}
	}

	pub async fn lock_shared(file: &tokio::fs::File) -> Result<()> {
		let fd = file.as_raw_fd();
		let ret = tokio::task::spawn_blocking(move || unsafe { flock(fd, LOCK_SH) })
			.await
			.unwrap();
		if ret != 0 {
			bail!(Error::from(std::io::Error::last_os_error()).context("The flock syscall failed."));
		}
		Ok(())
	}

	pub async fn lock_exclusive(file: &tokio::fs::File) -> Result<()> {
		let fd = file.as_raw_fd();
		let ret = tokio::task::spawn_blocking(move || unsafe { flock(fd, LOCK_EX) })
			.await
			.unwrap();
		if ret != 0 {
			bail!(Error::from(std::io::Error::last_os_error()).context("The flock syscall failed."));
		}
		Ok(())
	}

	pub fn _unlock(file: &tokio::fs::File) -> Result<()> {
		let fd = file.as_raw_fd();
		let ret = unsafe { flock(fd, LOCK_UN) };
		if ret != 0 {
			bail!(Error::from(std::io::Error::last_os_error()).context("The flock syscall failed."));
		}
		Ok(())
	}
}
