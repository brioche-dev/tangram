use crate::{blob, resource, return_error, target, task};
use futures::{
	stream::{self, BoxStream},
	StreamExt,
};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_stream::wrappers::BroadcastStream;

pub type Id = crate::Id;

#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
pub struct Evaluation {
	#[tangram_serialize(id = 0)]
	pub children: Vec<self::Id>,

	#[tangram_serialize(id = 1)]
	pub log: blob::Id,

	#[tangram_serialize(id = 2)]
	pub result: Result<crate::Id>,
}

impl Evaluation {
	pub fn serialize(&self) -> crate::Result<Vec<u8>> {
		let mut bytes = Vec::new();
		byteorder::WriteBytesExt::write_u8(&mut bytes, 0)?;
		tangram_serialize::to_writer(self, &mut bytes)?;
		Ok(bytes)
	}

	pub fn deserialize(mut bytes: &[u8]) -> crate::Result<Self> {
		let version = byteorder::ReadBytesExt::read_u8(&mut bytes)?;
		if version != 0 {
			return_error!(r#"Cannot deserialize a value with version "{version}"."#);
		}
		let value = tangram_serialize::from_reader(bytes)?;
		Ok(value)
	}
}

/// An evaluation result.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An evaluation error.
#[derive(
	Clone,
	Debug,
	Error,
	serde::Serialize,
	serde::Deserialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Error {
	/// An error from a resource.
	#[error(transparent)]
	#[tangram_serialize(id = 0)]
	Resource(#[from] resource::Error),

	/// An error from a target.
	#[error(transparent)]
	#[tangram_serialize(id = 1)]
	Target(#[from] target::Error),

	/// An error from a task.
	#[error(transparent)]
	#[tangram_serialize(id = 2)]
	Task(#[from] task::Error),

	/// A cancellation.
	#[error("The run was cancelled.")]
	#[tangram_serialize(id = 3)]
	Cancellation(()),
}

#[derive(Debug)]
pub struct State {
	task: Option<tokio::task::JoinHandle<Evaluation>>,
	children: std::sync::Mutex<(Vec<self::Id>, tokio::sync::broadcast::Sender<self::Id>)>,
	log: tokio::sync::Mutex<(tokio::fs::File, tokio::sync::broadcast::Sender<Vec<u8>>)>,
	result: (
		tokio::sync::watch::Sender<Option<Result<crate::Id>>>,
		tokio::sync::watch::Receiver<Option<Result<crate::Id>>>,
	),
}

impl State {
	pub fn new() -> crate::Result<Self> {
		let (children_tx, _) = tokio::sync::broadcast::channel(1024);
		let (log_tx, _) = tokio::sync::broadcast::channel(1024);
		let (result_tx, result_rx) = tokio::sync::watch::channel(None);
		let log_file = tokio::fs::File::from_std(tempfile::tempfile()?);
		Ok(Self {
			task: None,
			children: std::sync::Mutex::new((Vec::new(), children_tx)),
			log: tokio::sync::Mutex::new((log_file, log_tx)),
			result: (result_tx, result_rx),
		})
	}

	pub fn add_child(&self, id: self::Id) {
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

	pub fn set_result(&self, result: Result<crate::Id>) {
		self.result.0.send(Some(result)).ok();
	}

	pub fn children(&self) -> BoxStream<'static, self::Id> {
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

	pub async fn result(&self) -> Result<crate::Id> {
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
