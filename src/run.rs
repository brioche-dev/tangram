use crate::{blob, id, object, value, Blob, Client, Result, Value};
use futures::{
	stream::{self, BoxStream},
	StreamExt, TryStreamExt,
};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_stream::wrappers::BroadcastStream;

mod js;

crate::id!(Run);
crate::handle!(Run);
crate::data!();

#[derive(Clone, Copy, Debug)]
pub struct Id(crate::Id);

#[derive(Clone, Debug)]
pub struct Run(object::Handle);

#[derive(Clone, Debug)]
pub enum Object {
	Uncompleted,
	Completed(Completed),
}

#[derive(Clone, Debug)]
pub struct Completed {
	/// The run's children.
	children: Vec<Run>,

	/// The run's log.
	log: Blob,

	/// The run's result.
	result: Result<Value>,
}

#[derive(
	Clone,
	Debug,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
pub enum Data {
	#[tangram_serialize(id = 0)]
	Uncompleted(()),

	#[tangram_serialize(id = 1)]
	Completed(data::Completed),
}

pub mod data {
	use super::{blob, value, Id, Result};

	#[derive(
		Clone,
		Debug,
		serde::Deserialize,
		serde::Serialize,
		tangram_serialize::Deserialize,
		tangram_serialize::Serialize,
	)]
	pub struct Completed {
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
}

#[derive(Debug)]
pub struct State {
	/// The run's children.
	children: std::sync::Mutex<(Vec<Run>, tokio::sync::broadcast::Sender<Result<Run>>)>,

	/// The run's log.
	log: tokio::sync::Mutex<(
		tokio::fs::File,
		tokio::sync::broadcast::Sender<Result<Vec<u8>>>,
	)>,

	/// The run's result.
	#[allow(clippy::type_complexity)]
	result: (
		tokio::sync::watch::Sender<Option<Result<Value>>>,
		tokio::sync::watch::Receiver<Option<Result<Value>>>,
	),
}

impl Run {
	#[allow(clippy::new_without_default)]
	#[must_use]
	pub fn new() -> Self {
		Self(object::Handle::with_state((
			Some(Id::new().into()),
			Some(Object::Uncompleted.into()),
		)))
	}

	pub async fn children(&self, client: &Client) -> Result<BoxStream<'static, Result<Self>>> {
		let object = self.object(client).await?;
		match object {
			Object::Uncompleted => Ok(client
				.get_run_children(self.expect_id())
				.await?
				.map_ok(Run::with_id)
				.boxed()),
			Object::Completed(object) => Ok(stream::iter(object.children.clone()).map(Ok).boxed()),
		}
	}

	pub async fn log(&self, client: &Client) -> Result<BoxStream<'static, Result<Vec<u8>>>> {
		let object = self.object(client).await?;
		match object {
			Object::Uncompleted => Ok(client.get_run_log(self.expect_id()).await?.boxed()),
			Object::Completed(object) => {
				let log = object.log.clone();
				let client = client.clone();
				Ok(stream::once(async move { log.bytes(&client).await }).boxed())
			},
		}
	}

	pub async fn result(&self, client: &Client) -> Result<Result<Value>> {
		let object = self.object(client).await?;
		match object {
			Object::Uncompleted => Ok(client.get_run_result(self.expect_id()).await?),
			Object::Completed(object) => Ok(object.result.clone()),
		}
	}
}

impl Id {
	#[allow(clippy::new_without_default)]
	#[must_use]
	pub fn new() -> Self {
		Self(crate::Id::new_random(id::Kind::Run))
	}
}

impl Object {
	#[must_use]
	pub(crate) fn to_data(&self) -> Data {
		match self {
			Self::Uncompleted => Data::Uncompleted(()),
			Self::Completed(completed) => {
				let children = completed.children.iter().map(Run::expect_id).collect();
				let log = completed.log.expect_id();
				let result = completed.result.clone().map(|value| value.to_data());
				Data::Completed(data::Completed {
					children,
					log,
					result,
				})
			},
		}
	}

	#[must_use]
	pub(crate) fn from_data(data: Data) -> Self {
		match data {
			Data::Uncompleted(()) => Self::Uncompleted,
			Data::Completed(data) => {
				let children = data.children.into_iter().map(Run::with_id).collect();
				let log = Blob::with_id(data.log);
				let result = data.result.map(value::Value::from_data);
				let completed = Completed {
					children,
					log,
					result,
				};
				Self::Completed(completed)
			},
		}
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Handle> {
		match self {
			Self::Uncompleted => Vec::new(),
			Self::Completed(completed) => {
				let children = completed
					.children
					.iter()
					.map(|child| child.handle().clone());
				let log = std::iter::once(completed.log.handle().clone());
				let result = completed
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
			},
		}
	}
}

impl Data {
	#[must_use]
	pub fn children(&self) -> Vec<object::Id> {
		match self {
			Self::Uncompleted(()) => Vec::new(),
			Self::Completed(data) => {
				let children = data.children.iter().copied().map(Into::into);
				let log = std::iter::once(data.log.into());
				let result = data
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
			},
		}
	}
}

impl State {
	pub fn new() -> Result<Self> {
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

	pub fn add_child(&self, child: Run) {
		let mut children = self.children.lock().unwrap();
		children.0.push(child.clone());
		children.1.send(Ok(child)).ok();
	}

	pub async fn add_log(&self, bytes: Vec<u8>) -> Result<()> {
		let mut log = self.log.lock().await;
		log.0.seek(std::io::SeekFrom::End(0)).await?;
		log.0.write_all(&bytes).await?;
		log.1.send(Ok(bytes)).ok();
		Ok(())
	}

	pub fn set_result(&self, result: Result<Value>) {
		self.result.0.send(Some(result)).ok();
	}

	pub fn children(&self) -> BoxStream<'static, Result<Run>> {
		let children = self.children.lock().unwrap();
		let old = children.0.clone().into_iter().map(Ok);
		let new = BroadcastStream::new(children.1.subscribe())
			.filter_map(|result| async move { result.ok() });
		let stream = stream::iter(old).chain(new);
		stream.boxed()
	}

	pub async fn log(&self) -> Result<BoxStream<'static, Result<Vec<u8>>>> {
		let mut log = self.log.lock().await;
		log.0.seek(std::io::SeekFrom::Start(0)).await?;
		let mut old = Vec::new();
		log.0.read_to_end(&mut old).await?;
		log.0.seek(std::io::SeekFrom::End(0)).await?;
		let new =
			BroadcastStream::new(log.1.subscribe()).filter_map(|result| async move { result.ok() });
		let stream = stream::once(async move { Ok(old) }).chain(new);
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
