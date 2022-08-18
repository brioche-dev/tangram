#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

use std::fmt;

pub mod agent;
pub mod balloon_monitor;
mod bound_task;
pub mod cloud_init;
pub mod mem_info;
pub mod qemu;
pub mod systemd_unit;
pub mod template;

#[cfg(target_os = "macos")]
pub mod macos;

// For now, `machine` only supports the `Virtualization.framework` backend.
#[cfg(target_os = "macos")]
pub mod machine;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Writability {
	ReadOnly,
	ReadWrite,
}

impl Writability {
	/// Convert to the string `ro` or the string `rw`.
	#[must_use]
	pub fn as_ro_rw(&self) -> &'static str {
		match self {
			Writability::ReadOnly => "ro",
			Writability::ReadWrite => "rw",
		}
	}
}

impl fmt::Display for Writability {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Writability::ReadWrite => write!(f, "Read-Write"),
			Writability::ReadOnly => write!(f, "Read-Only"),
		}
	}
}
