use crossterm as ct;
use futures::{select, stream::BoxStream, StreamExt};
use tangram_client as tg;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug)]
pub enum Event {
	Terminal(ct::event::Event),
	Child(ChildEvent),
	Completed,
	Log(Vec<u8>),
}

#[derive(Debug)]
pub struct ChildEvent {
	pub parent: tg::build::Id,
	pub child: tg::build::Id,
}

pub struct ChildListener {
	receiver: tokio::sync::mpsc::Receiver<tg::Build>,
	task: tokio::task::JoinHandle<()>,
}

impl ChildListener {
	pub fn new(client: &dyn tg::Client, build: tg::build::Id) -> Self {
		let build = tg::Build::with_id(build);
		let (sender, receiver) = tokio::sync::mpsc::channel(512);
		let client = client.clone_box();
		let task = tokio::task::spawn(async move {
			let Ok(log) = build.children(client.as_mut()).await else {
				tracing::error!("Failed to get build children.");
				return;
			};
			while let Some(Ok(next)) = log.next().await {
				sender.send(next).await;
			}
		});
		Self { receiver, task }
	}
}

pub struct LogListener {
	receiver: tokio::sync::mpsc::Receiver<Vec<u8>>,
	task: tokio::task::JoinHandle<()>,
}

impl Drop for LogListener {
	fn drop(&mut self) {
		self.task.abort();
	}
}

impl LogListener {
	pub fn new(client: &dyn tg::Client, build: tg::build::Id) -> Self {
		let build = tg::Build::with_id(build);
		let (sender, receiver) = tokio::sync::mpsc::channel(512);
		let client = client.clone_box();
		let task = tokio::task::spawn(async move {
			let Ok(log) = build.log(client.as_mut()).await else {
				tracing::error!("Failed to create log.");
				return;
			};
			while let Some(Ok(next)) = log.next().await {
				sender.send(next.to_vec()).await;
			}
		});
		Self { receiver, task }
	}
}

pub struct EventStream {
	log_sender: UnboundedSender<tg::Build>,
	event_receiver: UnboundedReceiver<Vec<Event>>,
	task: tokio::task::JoinHandle<()>,
}

impl EventStream {
	pub fn new(timeout: std::time::Duration, client: &dyn tg::Client, build: tg::Build) -> Self {
		let (log_sender, log_receiver) = tokio::sync::mpsc::unbounded_channel::<tg::Build>();
		let (event_sender, event_receiver) = tokio::sync::mpsc::unbounded_channel();
		let client = client.clone_box();
		let task = tokio::task::spawn(async move {
			let Ok(log) = build.log(client.as_ref()).await else {
				tracing::error!("Failed to get the log stream of the root build.");
				return;
			};
			let mut log = log.fuse();

			let Ok(children) = child_stream(client.as_ref(), build).await else {
				tracing::error!("Failed to get the children of the root build.");
				return;
			};
			let children = children.fuse();
			let mut children = futures::stream::select_all([children]);

			loop {
				if let Ok(build) = log_receiver.try_recv() {
					if let Ok(new_log) = build.log(client.as_ref()).await {
						log = new_log.fuse();
					}
				}
				let mut events = vec![];
				let _ = tokio::time::timeout(timeout, async {
					loop {
						select! {
							child = children.next() => {
								// TODO handle errors.
								if let Some(child) = child {
									// Add the children of this build to the event stream.
									let child_build = tg::Build::with_id(child.child.clone());
									if let Ok(grandchildren) = child_stream(client.as_ref(), child_build).await {
										children.push(grandchildren.fuse());
									}
									events.push(Event::Child(child));
								}
							}
							log = log.next() => {
								// TODO handle errors.
								if let Some(Ok(log)) = log {
									events.push(Event::Log(log.to_vec()));
								}
							}
						};
					}
				})
				.await;
				if let Err(_) = event_sender.send(events) {
					break;
				}
			}
		});

		Self {
			log_sender,
			event_receiver,
			task,
		}
	}

	pub fn poll(&self) -> Vec<Event> {
		self.event_receiver.try_recv().unwrap_or_default()
	}

	pub fn set_log(&self, log: tg::Build) {
		self.log_sender.send(log);
	}
}

impl Drop for EventStream {
	fn drop(&mut self) {
		self.event_receiver.close();
		self.task.abort();
	}
}

async fn child_stream(
	client: &dyn tg::Client,
	build: tg::Build,
) -> tg::Result<BoxStream<ChildEvent>> {
	let id = build.id().clone();
	let children = build.children(client).await?;
	let stream = children
		.filter_map(move |child| async move {
			let parent = id.clone();
			let child = child.ok()?.id();
			Some(ChildEvent { parent, child })
		})
		.boxed();
	Ok(stream)
}
