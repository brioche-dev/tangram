use crate::Server;
use std::sync::{atomic::AtomicBool, Arc};
use tangram_error::Result;

#[derive(Clone)]
pub struct Builder {
	inner: Arc<Inner>,
}

struct Inner {
	server: Server,
	stop: AtomicBool,
	task: Task,
}

type Task = (
	std::sync::Mutex<Option<tokio::task::JoinHandle<Result<()>>>>,
	std::sync::Mutex<Option<tokio::task::AbortHandle>>,
);

impl Builder {
	pub async fn start(server: &Server) -> Result<Self> {
		let stop = AtomicBool::new(false);
		let task = (std::sync::Mutex::new(None), std::sync::Mutex::new(None));
		let builder = Self {
			inner: Arc::new(Inner {
				server: server.clone(),
				stop,
				task,
			}),
		};
		let task = tokio::spawn({
			let builder = builder.clone();
			async move { builder.run().await }
		});
		let abort = task.abort_handle();
		builder.inner.task.0.lock().unwrap().replace(task);
		builder.inner.task.1.lock().unwrap().replace(abort);
		Ok(builder)
	}

	pub fn stop(&self) {
		let ordering = std::sync::atomic::Ordering::SeqCst;
		self.inner.stop.store(true, ordering);
	}

	pub async fn join(&self) -> Result<()> {
		// Join the task.
		let task = self.inner.task.0.lock().unwrap().take();
		if let Some(task) = task {
			match task.await {
				Ok(result) => Ok(result),
				Err(error) if error.is_cancelled() => Ok(Ok(())),
				Err(error) => Err(error),
			}
			.unwrap()?;
		}

		Ok(())
	}

	pub async fn run(&self) -> Result<()> {
		let server = &self.inner.server;
		while !self.inner.stop.load(std::sync::atomic::Ordering::SeqCst) {
			let Some(build_id) = server.get_build_from_queue().await.ok() else {
				continue;
			};
			server.start_build(&build_id).await?;
		}
		Ok(())
	}
}
