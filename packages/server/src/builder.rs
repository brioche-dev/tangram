use crate::Server;
use std::sync::Arc;
use tangram_client as tg;
use tangram_error::Result;

#[derive(Clone)]
pub struct Builder {
	inner: Arc<Inner>,
}

struct Inner {
	server: Server,
	stop_sender: tokio::sync::watch::Sender<bool>,
	stop_receiver: tokio::sync::watch::Receiver<bool>,
	task: Task,
	systems: Option<Vec<tg::System>>,
}

type Task = (
	std::sync::Mutex<Option<tokio::task::JoinHandle<Result<()>>>>,
	std::sync::Mutex<Option<tokio::task::AbortHandle>>,
);

pub struct Options {
	pub systems: Option<Vec<tg::System>>,
}

impl Builder {
	pub fn start(server: &Server, options: Options) -> Self {
		let (stop_sender, stop_receiver) = tokio::sync::watch::channel(false);
		let task = (std::sync::Mutex::new(None), std::sync::Mutex::new(None));
		let builder = Self {
			inner: Arc::new(Inner {
				server: server.clone(),
				stop_receiver,
				stop_sender,
				task,
				systems: options.systems,
			}),
		};
		let task = tokio::spawn({
			let builder = builder.clone();
			async move { builder.run().await }
		});
		let abort = task.abort_handle();
		builder.inner.task.0.lock().unwrap().replace(task);
		builder.inner.task.1.lock().unwrap().replace(abort);
		builder
	}

	pub fn stop(&self) {
		self.inner.stop_sender.send(true).unwrap();
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
		let mut stop_receiver = self.inner.stop_receiver.clone();
		let server = self.inner.server.clone();
		loop {
			let result = tokio::select! {
				_ = stop_receiver.wait_for(|s| *s) => return Ok(()),
				result = server.get_build_from_queue(None, None) => result,
			};
			let queue_item = match result {
				Ok(queue_item) => queue_item,
				Err(error) => {
					tracing::error!(?error, "Failed to get a build from queue.");
					tokio::time::sleep(std::time::Duration::from_secs(1)).await;
					continue;
				},
			};
			server.start_build(None, &queue_item.build, queue_item.retry);
		}
	}
}
