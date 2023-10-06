use crate::{blob, id, object, return_error, value, Blob, Client, Error, Result, Value, WrapErr};
use futures::{
	stream::{self, BoxStream},
	StreamExt, TryStreamExt,
};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_stream::wrappers::BroadcastStream;

pub mod js;

crate::id!(Run);

#[derive(Clone, Copy, Debug)]
pub struct Id(crate::Id);

#[derive(Clone, Debug)]
pub struct Run(object::Handle);

#[derive(Clone, Debug)]
pub struct Object {
	/// The run's children.
	pub children: Vec<Run>,

	/// The run's log.
	pub log: Blob,

	/// The run's output.
	pub output: Option<Value>,
}

#[derive(
	Clone,
	Debug,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
pub struct Data {
	/// The run's children.
	#[tangram_serialize(id = 0)]
	pub children: Vec<Id>,

	/// The run's log.
	#[tangram_serialize(id = 1)]
	pub log: blob::Id,

	/// The run's output.
	#[tangram_serialize(id = 2)]
	pub output: Option<value::Data>,
}

#[allow(clippy::type_complexity)]
#[derive(Debug)]
pub struct State {
	/// The run's children.
	children: std::sync::Mutex<(Vec<Run>, Option<tokio::sync::broadcast::Sender<Run>>)>,

	/// The run's log.
	log: Arc<
		tokio::sync::Mutex<(
			tokio::fs::File,
			Option<tokio::sync::broadcast::Sender<Vec<u8>>>,
		)>,
	>,

	/// The run's output.
	output: (
		tokio::sync::watch::Sender<Option<Option<Value>>>,
		tokio::sync::watch::Receiver<Option<Option<Value>>>,
	),
}

impl Run {
	#[must_use]
	pub fn with_id(id: Id) -> Self {
		Self(object::Handle::with_id(id.into()))
	}

	#[must_use]
	pub fn with_object(object: Object) -> Self {
		Self(object::Handle::with_object(object.into()))
	}

	#[must_use]
	pub fn id(&self) -> Id {
		self.0.expect_id().try_into().unwrap()
	}

	#[must_use]
	pub fn handle(&self) -> &object::Handle {
		&self.0
	}

	pub async fn try_get_object(&self, client: &Client) -> Result<Option<&Object>> {
		match self.0.try_get_object(client).await? {
			Some(object::Object::Run(object)) => Ok(Some(object)),
			None => Ok(None),
			_ => unreachable!(),
		}
	}

	pub async fn children(&self, client: &Client) -> Result<BoxStream<'static, Self>> {
		self.try_get_children(client)
			.await?
			.wrap_err("Failed to get the run.")
	}

	pub async fn try_get_children(
		&self,
		client: &Client,
	) -> Result<Option<BoxStream<'static, Self>>> {
		if let Some(object) = self.try_get_object(client).await? {
			Ok(Some(stream::iter(object.children.clone()).boxed()))
		} else {
			Ok(client
				.try_get_run_children(self.id())
				.await?
				.map(|children| children.map(Run::with_id).boxed()))
		}
	}

	pub async fn log(&self, client: &Client) -> Result<BoxStream<'static, Vec<u8>>> {
		self.try_get_log(client)
			.await?
			.wrap_err("Failed to get the run.")
	}

	pub async fn try_get_log(
		&self,
		client: &Client,
	) -> Result<Option<BoxStream<'static, Vec<u8>>>> {
		if let Some(object) = self.try_get_object(client).await? {
			let log = object.log.clone();
			let client = client.clone();
			let bytes = log.bytes(&client).await?;
			Ok(Some(stream::once(async move { bytes }).boxed()))
		} else {
			Ok(client.try_get_run_log(self.id()).await?)
		}
	}

	pub async fn output(&self, client: &Client) -> Result<Option<Value>> {
		self.try_get_output(client)
			.await?
			.wrap_err("Failed to get the run.")
	}

	pub async fn try_get_output(&self, client: &Client) -> Result<Option<Option<Value>>> {
		if let Some(object) = self.try_get_object(client).await? {
			Ok(Some(object.output.clone()))
		} else {
			Ok(client.try_get_run_output(self.id()).await?)
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
		let children = self.children.iter().map(Run::id).collect();
		let log = self.log.expect_id();
		let output = self.output.clone().map(|value| value.to_data());
		Data {
			children,
			log,
			output,
		}
	}

	#[must_use]
	pub(crate) fn from_data(data: Data) -> Self {
		let children = data.children.into_iter().map(Run::with_id).collect();
		let log = Blob::with_id(data.log);
		let output = data.output.map(value::Value::from_data);
		Self {
			children,
			log,
			output,
		}
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Handle> {
		let children = self
			.children
			.iter()
			.map(|child| object::Handle::with_id(child.id().into()));
		let log = std::iter::once(self.log.handle().clone());
		let output = self
			.output
			.as_ref()
			.map(Value::children)
			.into_iter()
			.flatten();
		std::iter::empty()
			.chain(children)
			.chain(log)
			.chain(output)
			.collect()
	}
}

impl Data {
	pub(crate) fn serialize(&self) -> Result<Vec<u8>> {
		let mut bytes = Vec::new();
		byteorder::WriteBytesExt::write_u8(&mut bytes, 0)?;
		tangram_serialize::to_writer(self, &mut bytes)?;
		Ok(bytes)
	}

	pub(crate) fn deserialize(mut bytes: &[u8]) -> Result<Self> {
		let version = byteorder::ReadBytesExt::read_u8(&mut bytes)?;
		if version != 0 {
			return_error!(r#"Cannot deserialize this object with version "{version}"."#);
		}
		let value = tangram_serialize::from_reader(bytes)?;
		Ok(value)
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Id> {
		let children = self.children.iter().copied().map(Into::into);
		let log = std::iter::once(self.log.into());
		let output = self
			.output
			.as_ref()
			.map(value::Data::children)
			.into_iter()
			.flatten();
		std::iter::empty()
			.chain(children)
			.chain(log)
			.chain(output)
			.collect()
	}
}

impl State {
	pub fn new() -> Result<Self> {
		let (children_tx, _) = tokio::sync::broadcast::channel(1024);
		let (log_tx, _) = tokio::sync::broadcast::channel(1024);
		let (result_tx, result_rx) = tokio::sync::watch::channel(None);
		let log_file = tokio::fs::File::from_std(tempfile::tempfile()?);
		Ok(Self {
			children: std::sync::Mutex::new((Vec::new(), Some(children_tx))),
			log: Arc::new(tokio::sync::Mutex::new((log_file, Some(log_tx)))),
			output: (result_tx, result_rx),
		})
	}

	pub fn add_child(&self, child: Run) {
		let mut children = self.children.lock().unwrap();
		children.0.push(child.clone());
		children.1.as_ref().unwrap().send(child).ok();
	}

	pub fn add_log(&self, bytes: Vec<u8>) {
		tokio::spawn({
			let log = self.log.clone();
			async move {
				let mut log = log.lock().await;
				log.0.seek(std::io::SeekFrom::End(0)).await.ok();
				log.0.write_all(&bytes).await.ok();
				log.1.as_ref().unwrap().send(bytes).ok();
			}
		});
	}

	pub async fn set_output(&self, output: Option<Value>) {
		// Set the result.
		self.output.0.send(Some(output)).unwrap();

		// End the children and log streams.
		self.children.lock().unwrap().1.take();
		self.log.lock().await.1.take();
	}

	pub fn children(&self) -> BoxStream<'static, Run> {
		let children = self.children.lock().unwrap();
		let old = stream::iter(children.0.clone());
		let new = if let Some(new) = children.1.as_ref() {
			BroadcastStream::new(new.subscribe())
				.filter_map(|result| async move { result.ok() })
				.boxed()
		} else {
			stream::empty().boxed()
		};
		old.chain(new).boxed()
	}

	pub async fn log(&self) -> Result<BoxStream<'static, Vec<u8>>> {
		let mut log = self.log.lock().await;
		log.0.rewind().await?;
		let mut old = Vec::new();
		log.0.read_to_end(&mut old).await?;
		let old = stream::once(async move { old });
		log.0.seek(std::io::SeekFrom::End(0)).await?;
		let new = if let Some(new) = log.1.as_ref() {
			BroadcastStream::new(new.subscribe())
				.filter_map(|result| async move { result.ok() })
				.boxed()
		} else {
			stream::empty().boxed()
		};
		Ok(old.chain(new).boxed())
	}

	pub async fn output(&self) -> Option<Value> {
		self.output
			.1
			.clone()
			.wait_for(Option::is_some)
			.await
			.unwrap()
			.clone()
			.unwrap()
	}
}
