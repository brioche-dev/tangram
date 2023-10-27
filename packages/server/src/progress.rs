use bytes::Bytes;
use futures::{
	stream::{self, BoxStream},
	StreamExt, TryStreamExt,
};
use std::sync::Arc;
use tangram_client as tg;
use tg::{Result, Wrap, WrapErr};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_stream::wrappers::BroadcastStream;

#[derive(Clone, Debug)]
pub struct Progress {
	state: Arc<State>,
}

#[derive(Debug)]
struct State {
	id: tg::build::Id,
	target: tg::Target,
	children: std::sync::Mutex<ChildrenState>,
	log: Arc<tokio::sync::Mutex<LogState>>,
	logger: std::sync::Mutex<Option<tokio::sync::mpsc::UnboundedSender<Bytes>>>,
	logger_task: std::sync::Mutex<Option<tokio::task::JoinHandle<Result<()>>>>,
	result: ResultState,
}

#[derive(Debug)]
struct ChildrenState {
	children: Vec<tg::Build>,
	sender: Option<tokio::sync::broadcast::Sender<tg::Build>>,
}

#[derive(Debug)]
struct LogState {
	file: tokio::fs::File,
	sender: Option<tokio::sync::broadcast::Sender<Bytes>>,
}

#[derive(Debug)]
struct ResultState {
	sender: tokio::sync::watch::Sender<Option<Result<tg::Value>>>,
	receiver: tokio::sync::watch::Receiver<Option<Result<tg::Value>>>,
}

impl Progress {
	pub fn new(id: tg::build::Id, target: tg::Target) -> Result<Self> {
		// Create the children state.
		let children = std::sync::Mutex::new(ChildrenState {
			children: Vec::new(),
			sender: Some(tokio::sync::broadcast::channel(1024).0),
		});

		// Create the log state.
		let log = Arc::new(tokio::sync::Mutex::new(LogState {
			file: tokio::fs::File::from_std(
				tempfile::tempfile().wrap_err("Failed to create the temporary file.")?,
			),
			sender: Some(tokio::sync::broadcast::channel(1024).0),
		}));

		// Spawn the logger task.
		let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<Bytes>();
		let logger = std::sync::Mutex::new(Some(sender));
		let logger_task = std::sync::Mutex::new(Some(tokio::spawn({
			let log = log.clone();
			async move {
				while let Some(bytes) = receiver.recv().await {
					let mut log = log.lock().await;
					log.file
						.seek(std::io::SeekFrom::End(0))
						.await
						.wrap_err("Failed to seek.")?;
					log.file
						.write_all(&bytes)
						.await
						.wrap_err("Failed to write the log.")?;
					log.sender.as_ref().unwrap().send(bytes).ok();
				}
				Ok(())
			}
		})));

		// Create the result state.
		let (sender, receiver) = tokio::sync::watch::channel(None);
		let result = ResultState { sender, receiver };

		Ok(Self {
			state: Arc::new(State {
				id,
				target,
				children,
				log,
				logger,
				logger_task,
				result,
			}),
		})
	}

	pub fn target(&self) -> &tg::Target {
		&self.state.target
	}

	pub fn children_stream(&self) -> BoxStream<'static, Result<tg::Build>> {
		let state = self.state.children.lock().unwrap();
		let old = stream::iter(state.children.clone()).map(Ok);
		let new = if let Some(sender) = state.sender.as_ref() {
			BroadcastStream::new(sender.subscribe())
				.map_err(|err| err.wrap("Failed to create the stream."))
				.boxed()
		} else {
			stream::empty().boxed()
		};
		old.chain(new).boxed()
	}

	pub async fn log_stream(&self) -> Result<BoxStream<'static, Result<Bytes>>> {
		let mut log = self.state.log.lock().await;
		log.file
			.rewind()
			.await
			.wrap_err("Failed to rewind the log file.")?;
		let mut old = Vec::new();
		log.file
			.read_to_end(&mut old)
			.await
			.wrap_err("Failed to read the log.")?;
		let old = stream::once(async move { Ok(old.into()) });
		log.file
			.seek(std::io::SeekFrom::End(0))
			.await
			.wrap_err("Failed to seek in the log file.")?;
		let new = if let Some(sender) = log.sender.as_ref() {
			BroadcastStream::new(sender.subscribe())
				.map_err(|err| err.wrap("Failed to create the stream."))
				.boxed()
		} else {
			stream::empty().boxed()
		};
		Ok(old.chain(new).boxed())
	}

	pub async fn wait_for_result(&self) -> Result<tg::Value> {
		self.state
			.result
			.receiver
			.clone()
			.wait_for(Option::is_some)
			.await
			.unwrap()
			.clone()
			.unwrap()
	}

	pub async fn finish(self, client: &dyn tg::Client) -> Result<tg::Build> {
		// Drop the children sender.
		self.state.children.lock().unwrap().sender.take();

		// Drop the logger sender and wait for the logger task to finish.
		self.state.logger.lock().unwrap().take();
		let logger_task = self.state.logger_task.lock().unwrap().take().unwrap();
		logger_task.await.unwrap()?;

		// Get the children.
		let children = self.state.children.lock().unwrap().children.clone();

		// Get the log.
		let log = {
			let mut state = self.state.log.lock().await;
			state.file.rewind().await.wrap_err("Failed to seek.")?;
			tg::Blob::with_reader(client, &mut state.file).await?
		};

		// Get the result.
		let result = self.state.result.receiver.borrow().clone().unwrap();

		// Create the build.
		let build = tg::Build::new(
			client,
			self.state.id.clone(),
			self.state.target.clone(),
			children,
			log,
			result,
		)
		.await?;

		Ok(build)
	}
}

impl tangram_runtime::Progress for Progress {
	fn clone_box(&self) -> Box<dyn tangram_runtime::Progress> {
		Box::new(self.clone())
	}

	fn child(&self, child: &tg::Build) {
		let mut state = self.state.children.lock().unwrap();
		state.children.push(child.clone());
		state.sender.as_ref().unwrap().send(child.clone()).ok();
	}

	fn log(&self, bytes: Bytes) {
		eprint!("{}", std::str::from_utf8(&bytes).unwrap());
		self.state
			.logger
			.lock()
			.unwrap()
			.as_ref()
			.unwrap()
			.send(bytes)
			.unwrap();
	}

	fn result(&self, result: Result<tg::Value>) {
		self.state.result.sender.send(Some(result)).unwrap();
	}
}
