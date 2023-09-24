use crate::{blob, id, object, value, Blob, Client, Result, Value};
use futures::{
	stream::{self, BoxStream},
	StreamExt,
};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_stream::wrappers::BroadcastStream;

mod js;

#[derive(Clone, Debug)]
pub enum Run {
	Uncompleted(Arc<State>),
	Completed(object::Handle),
}

#[derive(Debug)]
pub struct State {
	id: Id,
	children: std::sync::Mutex<(Vec<Id>, tokio::sync::broadcast::Sender<Id>)>,
	log: tokio::sync::Mutex<(tokio::fs::File, tokio::sync::broadcast::Sender<Vec<u8>>)>,
	#[allow(clippy::type_complexity)]
	result: (
		tokio::sync::watch::Sender<Option<Result<Value>>>,
		tokio::sync::watch::Receiver<Option<Result<Value>>>,
	),
}

crate::object!(Run);

#[derive(Clone, Debug)]
pub struct Object {
	/// The run's children.
	pub children: Vec<Run>,

	/// The run's log.
	pub log: Blob,

	/// The run's result.
	pub result: Result<Value>,
}

#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
pub(crate) struct Data {
	/// The run's children.
	#[tangram_serialize(id = 0)]
	pub children: Vec<Id>,

	/// The run's log.
	#[tangram_serialize(id = 1)]
	pub log: blob::Id,

	/// The run's result.
	#[tangram_serialize(id = 2)]
	pub result: Result<value::Data>,
}

impl Run {
	#[must_use]
	pub fn with_id(id: Id) -> Self {
		Self::Completed(Handle::with_id(id))
	}

	pub fn with_state(state: Arc<State>) -> Self {
		Self::Uncompleted(state)
	}

	pub fn new() -> Self {
		Self::Uncompleted(Arc::new(State::new(Id::new()).unwrap()))
	}

	pub fn id(&self) -> Id {
		match self {
			Self::Uncompleted(state) => state.id,
			Self::Completed(run) => run.expect_id(),
		}
	}

	pub async fn children(&self, client: &Client) -> Result<BoxStream<'static, Self>> {
		todo!()
		// client
		// 	.get_run_children(self.expect_id())
		// 	.await
		// 	.map(|result| result.map(Self::with_id).boxed())
	}

	pub async fn log(&self, client: &Client) -> Result<BoxStream<'static, Vec<u8>>> {
		todo!()
		// client.get_run_log(self.expect_id()).await
	}

	pub async fn result(&self, client: &Client) -> Result<Result<Value>> {
		todo!()
		// client.get_run_result(self.expect_id()).await
	}
}

impl Id {
	#[must_use]
	pub fn new() -> Self {
		Self(crate::Id::new_random(id::Kind::Run))
	}
}

impl Object {
	#[must_use]
	pub(crate) fn to_data(&self) -> Data {
		Data {
			children: self.children.iter().map(Run::expect_id).collect(),
			log: self.log.handle().expect_id(),
			result: self.result.clone().map(|value| value.to_data()),
		}
	}

	#[must_use]
	pub(crate) fn from_data(data: Data) -> Self {
		Self {
			children: data.children.into_iter().map(Run::with_id).collect(),
			log: Blob::with_id(data.log),
			result: data.result.map(value::Value::from_data),
		}
	}

	pub fn children(&self) -> Vec<object::Handle> {
		let children = self.children.iter().cloned().map(Into::into);
		let log = std::iter::once(self.log.handle().clone().into());
		let result = self
			.result
			.as_ref()
			.ok()
			.map(Value::children)
			.into_iter()
			.flatten();
		std::iter::empty()
			.chain(children)
			.chain(log)
			.chain(result)
			.collect()
	}
}

impl Data {
	pub fn children(&self) -> Vec<object::Id> {
		let children = self.children.iter().copied().map(Into::into);
		let log = std::iter::once(self.log.into());
		let result = self
			.result
			.as_ref()
			.ok()
			.map(value::Data::children)
			.into_iter()
			.flatten();
		std::iter::empty()
			.chain(children)
			.chain(log)
			.chain(result)
			.collect()
	}
}

impl State {
	pub fn new(id: Id) -> Result<Self> {
		let (children_tx, _) = tokio::sync::broadcast::channel(1024);
		let (log_tx, _) = tokio::sync::broadcast::channel(1024);
		let (result_tx, result_rx) = tokio::sync::watch::channel(None);
		let log_file = tokio::fs::File::from_std(tempfile::tempfile()?);
		Ok(Self {
			id,
			children: std::sync::Mutex::new((Vec::new(), children_tx)),
			log: tokio::sync::Mutex::new((log_file, log_tx)),
			result: (result_tx, result_rx),
		})
	}

	pub fn add_child(&self, id: Id) {
		let mut children = self.children.lock().unwrap();
		children.0.push(id);
		children.1.send(id).ok();
	}

	pub async fn add_log(&self, bytes: Vec<u8>) -> Result<()> {
		let mut log = self.log.lock().await;
		log.0.seek(std::io::SeekFrom::End(0)).await?;
		log.0.write_all(&bytes).await?;
		log.1.send(bytes).ok();
		Ok(())
	}

	pub fn set_result(&self, result: Result<Value>) {
		self.result.0.send(Some(result)).ok();
	}

	pub fn children(&self) -> BoxStream<'static, Id> {
		let children = self.children.lock().unwrap();
		let old = children.0.clone();
		let new = BroadcastStream::new(children.1.subscribe())
			.filter_map(|result| async move { result.ok() });
		let stream = stream::iter(old).chain(new);
		stream.boxed()
	}

	pub async fn log(&self) -> Result<BoxStream<'static, Vec<u8>>> {
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

	pub async fn result(&self) -> Result<Value> {
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
