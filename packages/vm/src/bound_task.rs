//! Asynchronous task handle that aborts the task on drop.

use std::future::Future;
use tokio::task;

pub struct BoundJoinHandle<T> {
	inner: task::JoinHandle<T>,
}

/// Spawn a task which will be aborted when the [`BoundJoinHandle`] is dropped.
#[track_caller]
pub fn spawn<F>(future: F) -> BoundJoinHandle<F::Output>
where
	F: Future + Send + 'static,
	F::Output: Send + 'static,
{
	BoundJoinHandle {
		inner: task::spawn(future),
	}
}

impl<T> Drop for BoundJoinHandle<T> {
	/// On drop, abort the inner task.
	fn drop(&mut self) {
		self.inner.abort();
	}
}

impl<T> std::ops::Deref for BoundJoinHandle<T> {
	type Target = tokio::task::JoinHandle<T>;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}
