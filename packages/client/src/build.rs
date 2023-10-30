use crate::{blob, id, object, target, value, Blob, Client, Result, Target, Value, WrapErr};
use bytes::Bytes;
use futures::{
	stream::{self, BoxStream, FuturesUnordered},
	StreamExt, TryStreamExt,
};

crate::id!(Build);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Id(crate::Id);

#[derive(Clone, Debug)]
pub struct Build(object::Handle);

#[derive(Clone, Debug)]
pub struct Object {
	pub target: Target,
	pub children: Vec<Build>,
	pub log: Blob,
	pub result: Result<Value>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Data {
	pub target: target::Id,
	pub children: Vec<Id>,
	pub log: blob::Id,
	pub result: Result<value::Data>,
}

impl Id {
	#[allow(clippy::new_without_default)]
	#[must_use]
	pub fn new() -> Self {
		Self(crate::Id::new_random(id::Kind::Build))
	}
}

impl Build {
	pub async fn new(
		client: &dyn Client,
		id: Id,
		target: Target,
		children: Vec<Build>,
		log: Blob,
		result: Result<Value>,
	) -> Result<Self> {
		// Create the object.
		let object = Object {
			target,
			children,
			log,
			result,
		};

		// Store the children.
		object
			.children()
			.into_iter()
			.map(|child| async move { child.store(client).await })
			.collect::<FuturesUnordered<_>>()
			.try_collect()
			.await?;

		// Get the data.
		let data = object.to_data();

		// Serialize the data.
		let bytes = data.serialize()?;

		// Store the object.
		client
			.try_put_object_bytes(&id.clone().into(), &bytes)
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
	pub fn id(&self) -> &Id {
		match self.0.expect_id() {
			object::Id::Build(id) => id,
			_ => unreachable!(),
		}
	}

	#[must_use]
	pub fn handle(&self) -> &object::Handle {
		&self.0
	}

	pub async fn target(&self, client: &dyn Client) -> Result<Target> {
		self.try_get_target(client)
			.await?
			.wrap_err("Failed to get the build.")
	}

	pub async fn try_get_target(&self, client: &dyn Client) -> Result<Option<Target>> {
		match self.0.try_get_object(client).await? {
			Some(object::Object::Build(object)) => Ok(Some(object.target.clone())),
			None => Ok(None),
			_ => unreachable!(),
		}
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

	pub async fn result(&self, client: &dyn Client) -> Result<Result<Value>> {
		self.try_get_result(client)
			.await?
			.wrap_err("Failed to get the build.")
	}

	pub async fn try_get_result(&self, client: &dyn Client) -> Result<Option<Result<Value>>> {
		if let Some(object) = self.try_get_object(client).await? {
			Ok(Some(object.result.clone()))
		} else {
			Ok(client.try_get_build_result(self.id()).await?)
		}
	}
}

impl Object {
	#[must_use]
	pub fn to_data(&self) -> Data {
		let target = self.target.expect_id().clone();
		let children = self.children.iter().map(Build::id).cloned().collect();
		let log = self.log.expect_id().clone();
		let result = self.result.clone().map(|value| value.to_data());
		Data {
			target,
			children,
			log,
			result,
		}
	}

	#[must_use]
	pub fn from_data(data: Data) -> Self {
		let target = Target::with_id(data.target);
		let children = data.children.into_iter().map(Build::with_id).collect();
		let log = Blob::with_id(data.log);
		let result = data.result.map(Value::from_data);
		Self {
			target,
			children,
			log,
			result,
		}
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Handle> {
		let target = std::iter::once(self.target.handle().clone());
		let children = self.children.iter().map(|child| child.handle().clone());
		let log = std::iter::once(self.log.handle().clone());
		let result = self
			.result
			.as_ref()
			.map(Value::children)
			.into_iter()
			.flatten();
		std::iter::empty()
			.chain(target)
			.chain(children)
			.chain(log)
			.chain(result)
			.collect()
	}
}

impl Data {
	pub fn serialize(&self) -> Result<Bytes> {
		serde_json::to_vec(self)
			.map(Into::into)
			.wrap_err("Failed to serialize the data.")
	}

	pub fn deserialize(bytes: &Bytes) -> Result<Self> {
		serde_json::from_reader(bytes.as_ref()).wrap_err("Failed to deserialize the data.")
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Id> {
		let target = std::iter::once(self.target.clone().into());
		let children = self.children.iter().cloned().map(Into::into);
		let log = std::iter::once(self.log.clone().into());
		let result = self
			.result
			.as_ref()
			.map(value::Data::children)
			.into_iter()
			.flatten();
		std::iter::empty()
			.chain(target)
			.chain(children)
			.chain(log)
			.chain(result)
			.collect()
	}
}
