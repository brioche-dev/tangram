use crate::{bound_task, mem_info::MemInfo};
use std::{ops::RangeInclusive, time::Duration};
use ubyte::ByteUnit;

pub struct Config {
	/// Bounds on guest memory.
	pub mem_bounds: RangeInclusive<ByteUnit>,

	/// Amount of time to wait while polling the host for memory usage
	pub poll_interval: Duration,

	/// The minimum size of a guest memory adjustment (to avoid overly-frequent changes)
	pub threshold: ByteUnit,

	/// Rule to calculate the desired size of the guest.
	///
	/// The rule is supplied with the host memory info, and the current size of the guest, and
	/// returns the desired size of the guest.
	///
	/// The return value of the rule is bounded always by `mem_bounds`.
	pub rule: Box<dyn Fn(MemInfo, ByteUnit) -> ByteUnit + Send + 'static>,

	/// Closure to get the amount of memory currently allocated to the guest
	pub get_guest_mem: Box<dyn Fn() -> ByteUnit + Send + 'static>,

	/// Closure to set the amount of memory currently allocated to the guest.
	pub set_guest_mem: Box<dyn Fn(ByteUnit) + Send + 'static>,
}

/// A relative adjustment to the amount of guest memory.
#[derive(Clone, Copy)]
pub enum MemAdjustment {
	CanGrow,
	ShouldShrink,
}

pub struct BalloonMonitor {
	_task: bound_task::BoundJoinHandle<()>,
}

impl BalloonMonitor {
	/// Start monitoring for memory changes.
	#[must_use]
	pub fn start(config: Config) -> BalloonMonitor {
		BalloonMonitor {
			_task: bound_task::spawn(monitor(config)),
		}
	}
	/// Stop monitoring for memory changes.
	pub fn stop(self) {
		// Drop the _task handle, which kills the monitor task.
		drop(self);
	}
}

async fn monitor(config: Config) {
	loop {
		// Get the host memory stats
		let host_info = MemInfo::measure();

		// Get the guest memory
		let guest_mem = (config.get_guest_mem)();

		// Evaluate the memory rule to get a target guest memory size
		let target = (config.rule)(host_info, guest_mem);

		// Clamp the target to within the configured bounds
		let target = {
			let min = config.mem_bounds.start().as_u64();
			let max = config.mem_bounds.end().as_u64();
			let clamped = target.as_u64().clamp(min, max);
			ByteUnit::Byte(clamped)
		};

		// Set the guest memory to the target, if necessary.
		let difference = ByteUnit::Byte(target.as_u64().abs_diff(guest_mem.as_u64()));
		if difference >= config.threshold {
			(config.set_guest_mem)(target);
		}

		// Wait the specified interval before continuing.
		tokio::time::sleep(config.poll_interval).await;
	}
}
