use crate::{blob, resource, target, task, Id, Rid};
use futures::{
	stream::{self, BoxStream},
	StreamExt,
};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_stream::wrappers::BroadcastStream;

#[derive(Clone, Debug)]
pub struct Evaluation {
	children: Vec<Rid>,
	log: blob::Id,
	result: Result<Id>,
}

/// An evaluation result.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An evaluation error.
#[derive(Clone, Debug, Error, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Error {
	/// An error from a resource.
	#[error(transparent)]
	Resource(#[from] resource::Error),

	/// An error from a target.
	#[error(transparent)]
	Target(#[from] target::Error),

	/// An error from a task.
	#[error(transparent)]
	Task(#[from] task::Error),

	/// A cancellation.
	#[error("The run was cancelled.")]
	Cancellation,
}

#[derive(Debug)]
pub struct State {
	children: std::sync::Mutex<(Vec<Rid>, tokio::sync::broadcast::Sender<Rid>)>,
	log: tokio::sync::Mutex<(tokio::fs::File, tokio::sync::broadcast::Sender<Vec<u8>>)>,
	result: (
		tokio::sync::watch::Sender<Option<Result<Id>>>,
		tokio::sync::watch::Receiver<Option<Result<Id>>>,
	),
}

impl State {
	pub fn new() -> crate::Result<Self> {
		let (children_tx, _) = tokio::sync::broadcast::channel(1024);
		let (log_tx, _) = tokio::sync::broadcast::channel(1024);
		let (result_tx, result_rx) = tokio::sync::watch::channel(None);
		let log_file = tokio::fs::File::from_std(tempfile::tempfile()?);
		Ok(Self {
			children: std::sync::Mutex::new((Vec::new(), children_tx)),
			log: tokio::sync::Mutex::new((log_file, log_tx)),
			result: (result_tx, result_rx),
		})
	}

	pub fn add_child(&self, id: Rid) {
		let mut children = self.children.lock().unwrap();
		children.0.push(id);
		children.1.send(id).ok();
	}

	pub async fn add_log(&self, bytes: Vec<u8>) -> crate::Result<()> {
		let mut log = self.log.lock().await;
		log.0.seek(std::io::SeekFrom::End(0)).await?;
		log.0.write_all(&bytes).await?;
		log.1.send(bytes).ok();
		Ok(())
	}

	pub fn set_result(&self, result: Result<Id>) {
		self.result.0.send(Some(result)).ok();
	}

	pub fn children(&self) -> BoxStream<'static, Rid> {
		let children = self.children.lock().unwrap();
		let old = children.0.clone();
		let new = BroadcastStream::new(children.1.subscribe())
			.filter_map(|result| async move { result.ok() });
		let stream = stream::iter(old).chain(new);
		stream.boxed()
	}

	pub async fn log(&self) -> crate::Result<BoxStream<'static, Vec<u8>>> {
		let mut log = self.log.lock().await;
		log.0.seek(std::io::SeekFrom::Start(0)).await?;
		let mut old = Vec::new();
		log.0.read_to_end(&mut old).await?;
		log.0.seek(std::io::SeekFrom::End(0)).await?;
		let new =
			BroadcastStream::new(log.1.subscribe()).filter_map(|result| async move { result.ok() });
		let stream = stream::once(async move { old }).chain(new);
		Ok(stream.boxed())
	}

	pub async fn result(&self) -> Result<Id> {
		self.result
			.1
			.clone()
			.wait_for(Option::is_some)
			.await
			.unwrap()
			.clone()
			.unwrap()
	}
}
