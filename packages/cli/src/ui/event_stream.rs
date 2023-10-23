use std::{rc::Rc, sync::RwLock};

use futures::{select, stream::BoxStream, StreamExt};
use tangram_client as tg;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug)]
pub enum Event {
	Child(ChildEvent),
	Log(Vec<u8>),
	Completed(CompleteEvent),
}

#[derive(Debug)]
pub struct ChildEvent {
	pub parent: tg::Build,
	pub child: tg::Build,
}

#[derive(Debug)]
pub struct CompleteEvent {
	pub build: tg::Build,
	pub result: tg::Result<tg::Value>,
}

#[derive(Clone)]
pub struct EventStream {
	inner: Rc<RwLock<Inner>>,
}

struct Inner {
	log_sender: UnboundedSender<tg::Build>,
	event_receiver: UnboundedReceiver<Vec<Event>>,
	task: tokio::task::JoinHandle<()>,
}

impl EventStream {
	pub fn new(timeout: std::time::Duration, client: &dyn tg::Client, build: tg::Build) -> Self {
		let (log_sender, mut log_receiver) = tokio::sync::mpsc::unbounded_channel::<tg::Build>();
		let (event_sender, event_receiver) = tokio::sync::mpsc::unbounded_channel();
		let client = client.clone_box();
		let task = tokio::task::spawn(async move {
			// Create the log stream.
			let Ok(log) = build.log(client.as_ref()).await else {
				tracing::error!("Failed to get the log stream of the root build.");
				return;
			};
			let mut log = log.fuse();

			// Create the child build stream of the root build.
			let Ok(children) = child_stream(client.as_ref(), build.clone()).await else {
				tracing::error!("Failed to get the children of the root build.");
				return;
			};
			let children = children.fuse();

			// Create the completion event stream of the root.
			let completion = completion_stream(client.as_ref(), build.clone()).fuse();

			// Create a SelectAll over the child and completion event streams.
			let mut event_streams = futures::stream::select_all([children, completion]);
			loop {
				// First, see if the UI has requested us monitor another build's log.
				if let Ok(build) = log_receiver.try_recv() {
					if let Ok(new_log) = build.log(client.as_ref()).await {
						log = new_log.fuse();
					}
				}

				// Collect all the events that occur during the timeout.
				let mut events = vec![];
				let _ = tokio::time::timeout(timeout, async {
					loop {
						select! {
							// Handle new children and completions.
							event = event_streams.next() => {
								// TODO handle errors.
								if let Some(event) = event {
									if let Event::Child(event) = &event {
										let completion = completion_stream(client.as_ref(), event.child.clone());
										event_streams.push(completion.fuse());

										// Add the children of this build to the event stream.
										if let Ok(grandchildren) = child_stream(client.as_ref(), event.child.clone()).await {
											event_streams.push(grandchildren.fuse());

										}
									};
									events.push(event);
								}
							}

							// Handle any logs.
							log = log.next() => {
								// TODO handle errors.
								if let Some(Ok(log)) = log {
									println!("Pushing log: {log:#?}");
									events.push(Event::Log(log.to_vec()));
								}
							}

							// If all the streams have been completed, we're done and can exit.
							complete => {
								break
							}
						};
					}
				})
				.await;

				// If the event sender has been dropped, we can exit the main loop.
				if let Err(_) = event_sender.send(events) {
					break;
				}
			}
		});

		let inner = Rc::new(RwLock::new(Inner {
			log_sender,
			event_receiver,
			task,
		}));
		Self { inner }
	}

	pub fn poll(&self) -> Vec<Event> {
		let mut inner = self.inner.write().unwrap();
		inner.event_receiver.try_recv().unwrap_or_default()
	}

	pub fn set_log(&self, log: tg::Build) {
		let inner = self.inner.read().unwrap();
		let _ = inner.log_sender.send(log);
	}
}

impl Drop for Inner {
	fn drop(&mut self) {
		self.event_receiver.close();
		self.task.abort();
	}
}

async fn child_stream(client: &dyn tg::Client, build: tg::Build) -> tg::Result<BoxStream<Event>> {
	let id = build.id().clone();
	let children = build.children(client).await?;
	let stream = children
		.filter_map(move |child| async move {
			let parent = tg::Build::with_id(id);
			let child = child.ok()?;
			Some(Event::Child(ChildEvent { parent, child }))
		})
		.boxed();
	Ok(stream)
}

fn completion_stream(client: &dyn tg::Client, build: tg::Build) -> BoxStream<Event> {
	let client = client.clone_box();
	let stream = futures::stream::once(async move {
		let result = build.result(client.as_ref()).await.and_then(|res| res);
		let event = Event::Completed(CompleteEvent { build, result });
		event
	})
	.boxed();
	stream
}
