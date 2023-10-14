use crate::{blob, id, object, return_error, value, Blob, Client, Result, Value, WrapErr};
use bytes::Bytes;
use futures::{
	stream::{self, BoxStream},
	StreamExt, TryStreamExt,
};

crate::id!(Build);

#[derive(Clone, Copy, Debug)]
pub struct Id(crate::Id);

#[derive(Clone, Debug)]
pub struct Build(object::Handle);

#[derive(Clone, Debug)]
pub struct Object {
	pub children: Vec<Build>,
	pub log: Blob,
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
	#[tangram_serialize(id = 0)]
	pub children: Vec<Id>,

	#[tangram_serialize(id = 1)]
	pub log: blob::Id,

	#[tangram_serialize(id = 2)]
	pub output: Option<value::Data>,
}

impl Build {
	pub async fn new(
		client: &dyn Client,
		id: Id,
		children: Vec<Build>,
		log: Blob,
		output: Option<Value>,
	) -> Result<Self> {
		// Create the object.
		let object = Object {
			children,
			log,
			output,
		};

		// Store the children.
		object
			.children()
			.into_iter()
			.map(|child| async move { child.store(client).await })
			.collect::<futures::stream::FuturesUnordered<_>>()
			.try_collect()
			.await?;

		// Get the data.
		let data = object.to_data();

		// Serialize the data.
		let bytes = data.serialize()?;

		// Store the object.
		client
			.try_put_object_bytes(id.into(), &bytes)
			.await
			.wrap_err("Failed to put the object.")?
			.ok()
			.wrap_err("Expected all children to be stored.")?;

		Ok(Self(object::Handle::with_state(object::State::new(
			Some(id.into()),
			Some(object.into()),
		))))
	}

	#[must_use]
	pub fn with_id(id: Id) -> Self {
		Self(object::Handle::with_id(id.into()))
	}

	#[must_use]
	pub fn id(&self) -> Id {
		self.0.expect_id().try_into().unwrap()
	}

	#[must_use]
	pub fn handle(&self) -> &object::Handle {
		&self.0
	}

	pub async fn try_get_object(&self, client: &dyn Client) -> Result<Option<&Object>> {
		match self.0.try_get_object(client).await? {
			Some(object::Object::Build(object)) => Ok(Some(object)),
			None => Ok(None),
			_ => unreachable!(),
		}
	}

	pub async fn children(&self, client: &dyn Client) -> Result<BoxStream<'static, Result<Self>>> {
		self.try_get_children(client)
			.await?
			.wrap_err("Failed to get the build.")
	}

	pub async fn try_get_children(
		&self,
		client: &dyn Client,
	) -> Result<Option<BoxStream<'static, Result<Self>>>> {
		if let Some(object) = self.try_get_object(client).await? {
			Ok(Some(stream::iter(object.children.clone()).map(Ok).boxed()))
		} else {
			Ok(client
				.try_get_build_children(self.id())
				.await?
				.map(|children| children.map_ok(Build::with_id).boxed()))
		}
	}

	pub async fn log(&self, client: &dyn Client) -> Result<BoxStream<'static, Result<Bytes>>> {
		self.try_get_log(client)
			.await?
			.wrap_err("Failed to get the build.")
	}

	pub async fn try_get_log(
		&self,
		client: &dyn Client,
	) -> Result<Option<BoxStream<'static, Result<Bytes>>>> {
		if let Some(object) = self.try_get_object(client).await? {
			let log = object.log.clone();
			let bytes = log.bytes(client).await?;
			Ok(Some(stream::once(async move { Ok(bytes.into()) }).boxed()))
		} else {
			Ok(client.try_get_build_log(self.id()).await?)
		}
	}

	pub async fn output(&self, client: &dyn Client) -> Result<Option<Value>> {
		self.try_get_output(client)
			.await?
			.wrap_err("Failed to get the build.")
	}

	pub async fn try_get_output(&self, client: &dyn Client) -> Result<Option<Option<Value>>> {
		if let Some(object) = self.try_get_object(client).await? {
			Ok(Some(object.output.clone()))
		} else {
			Ok(client.try_get_build_output(self.id()).await?)
		}
	}
}

impl Id {
	#[allow(clippy::new_without_default)]
	#[must_use]
	pub fn new() -> Self {
		Self(crate::Id::new_random(id::Kind::Build))
	}
}

impl Object {
	#[must_use]
	pub fn to_data(&self) -> Data {
		let children = self.children.iter().map(Build::id).collect();
		let log = self.log.expect_id();
		let output = self.output.clone().map(|value| value.to_data());
		Data {
			children,
			log,
			output,
		}
	}

	#[must_use]
	pub fn from_data(data: Data) -> Self {
		let children = data.children.into_iter().map(Build::with_id).collect();
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
	pub fn serialize(&self) -> Result<Vec<u8>> {
		let mut bytes = Vec::new();
		byteorder::WriteBytesExt::write_u8(&mut bytes, 0)?;
		tangram_serialize::to_writer(self, &mut bytes)?;
		Ok(bytes)
	}

	pub fn deserialize(mut bytes: &[u8]) -> Result<Self> {
		let version = byteorder::ReadBytesExt::read_u8(&mut bytes)?;
		if version != 0 {
			return_error!(r#"Cannot deserialize with version "{version}"."#);
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
