use std::sync::{RwLock, Arc};

use futures::{
	stream::BoxStream,
	StreamExt,
};
use tangram_client as tg;
use tokio::sync::mpsc::UnboundedReceiver;

#[derive(Debug)]
pub enum Event {
	Child(ChildEvent),
	Completed(CompleteEvent),
}

#[derive(Debug)]
pub struct LogEvent {
	pub build: tg::Build,
	pub log: Vec<u8>,
}

#[derive(Debug)]
pub struct ChildEvent {
	pub parent: tg::Build,
	pub child: tg::Build,
	pub info: String,
}

#[derive(Debug)]
pub struct CompleteEvent {
	pub build: tg::Build,
	pub result: tg::Result<tg::Value>,
}

#[derive(Clone)]
pub struct EventStream {
	inner: Arc<RwLock<Inner>>,
}

struct Inner {
	event_receiver: UnboundedReceiver<Vec<Event>>,
	task: tokio::task::JoinHandle<()>,
}

impl EventStream {
	pub fn new(timeout: std::time::Duration, client: &dyn tg::Client, build: tg::Build) -> Self {
		let (event_sender, event_receiver) = tokio::sync::mpsc::unbounded_channel();
		let client = client.clone_box();
		let task = tokio::task::spawn(async move {
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
				// Collect all the events that occur during the timeout.
				let mut events = vec![];

				let _error = tokio::time::timeout(timeout, async {
					while let Some(event) = event_streams.next().await {
						if let Event::Child(event) = &event {
							let completion =
								completion_stream(client.as_ref(), event.child.clone());
							event_streams.push(completion.fuse());

							// Add the children of this build to the event stream.
							if let Ok(grandchildren) =
								child_stream(client.as_ref(), event.child.clone()).await
							{
								event_streams.push(grandchildren.fuse());
							}
						};
						events.push(event);
					}
				})
				.await;
				// If the event sender has been dropped, we can exit the main loop.
				if let Err(_) = event_sender.send(events) {
					println!("Closing event stream.");
					break;
				}
			}
		});

		let inner = Arc::new(RwLock::new(Inner {
			event_receiver,
			task,
		}));
		Self { inner }
	}

	pub fn poll(&self) -> Vec<Event> {
		let mut inner = self.inner.write().unwrap();
		inner.event_receiver.try_recv().unwrap_or_default()
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
			let client = client.clone_box();
			let parent = tg::Build::with_id(id);
			let child = child.ok()?;
			let info = info_string(client.as_ref(), &child).await;
			Some(Event::Child(ChildEvent {
				parent,
				child,
				info,
			}))
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

pub async fn info_string(client: &dyn tg::Client, build: &tg::Build) -> String {
	let target = match build.target(client).await {
		Ok(target) => target,
		Err(e) => return format!("Error: {e}"),
	};

	let name = match target.name(client).await {
		Ok(Some(name)) => name.clone(),
		Ok(None) => "<unknown>".into(),
		Err(e) => format!("Error: {e}"),
	};

	let package = match target.package(client).await {
		Ok(Some(package)) => match package.try_get_metadata(client).await {
			Ok(tg::package::Metadata { name, version }) => {
				let name = name.as_deref().unwrap_or("<unknown>");
				let version = version.as_deref().unwrap_or("<unknown>");
				format!("{name}@{version}")
			},
			Err(e) => format!("Error: {e}"),
		},
		Ok(None) => "<unknown>".into(),
		Err(e) => format!("Error: {e}"),
	};

	format!("{package}: {name}")
}
