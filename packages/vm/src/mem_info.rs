use lazy_static::lazy_static;
use std::fmt;
use std::sync::Mutex;
use sysinfo::{self, SystemExt};
use ubyte::{ByteUnit, ToByteUnit};

#[derive(Clone, Copy)]
pub struct MemInfo {
	pub total: ByteUnit,
	pub available: ByteUnit,
	pub free: ByteUnit,
	pub used: ByteUnit,
}

impl fmt::Debug for MemInfo {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		// Use Display impl for ByteUnit
		f.debug_struct("MemInfo")
			.field("total", &format!("{}", self.total))
			.field("available", &format!("{}", self.available))
			.field("free", &format!("{}", self.free))
			.field("used", &format!("{}", self.used))
			.finish()
	}
}

impl MemInfo {
	/// Measure the current host memory info.
	#[must_use]
	pub fn measure() -> MemInfo {
		lazy_static! {
			static ref SYSTEM_INFO: Mutex<sysinfo::System> = Mutex::new(sysinfo::System::new());
		}
		let mut system = SYSTEM_INFO.lock().expect("SYSTEM_INFO mutex was poisoned");

		// Refresh the sysinfo memory information
		system.refresh_memory();

		// Assemble memory info
		MemInfo {
			total: system.total_memory().kilobytes(),
			available: system.available_memory().kilobytes(),
			free: system.free_memory().kilobytes(),
			used: system.used_memory().kilobytes(),
		}
	}
}
