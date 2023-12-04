pub use self::data::Data;
use crate::{id, object, value, Blob, Client, Error, Result, Target, User, Value, WrapErr};
use async_recursion::async_recursion;
use bytes::Bytes;
use derive_more::{Display, TryUnwrap};
use futures::{
	stream::{self, BoxStream, FuturesOrdered},
	StreamExt, TryStreamExt,
};
use std::sync::Arc;
use tangram_error::return_error;

#[derive(
	Clone,
	Debug,
	Display,
	Eq,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	serde::Deserialize,
	serde::Serialize,
)]
#[serde(into = "crate::Id", try_from = "crate::Id")]
pub struct Id(crate::Id);

#[derive(Clone, Debug)]
pub struct Build {
	state: Arc<std::sync::RwLock<State>>,
}

type State = object::State<Id, Object>;

#[derive(Clone, Debug)]
pub struct Object {
	pub target: Target,
	pub children: Vec<Build>,
	pub log: Blob,
	pub outcome: Outcome,
}

#[derive(Clone, Debug, serde::Deserialize, TryUnwrap)]
#[serde(try_from = "data::Outcome")]
#[try_unwrap(ref)]
pub enum Outcome {
	Terminated,
	Canceled,
	Failed(Error),
	Succeeded(Value),
}

#[derive(
	Clone,
	Copy,
	Debug,
	Default,
	Eq,
	Ord,
	PartialEq,
	PartialOrd,
	serde::Deserialize,
	serde::Serialize,
)]
#[serde(into = "String", try_from = "String")]
pub enum Retry {
	Terminated,
	#[default]
	Canceled,
	Failed,
	Succeeded,
}

pub mod data {
	use super::Id;
	use crate::{blob, target, value};
	use derive_more::TryUnwrap;
	use tangram_error::Error;

	#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
	pub struct Data {
		pub target: target::Id,
		pub children: Vec<Id>,
		pub log: blob::Id,
		pub outcome: Outcome,
	}

	#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, TryUnwrap)]
	#[serde(rename_all = "camelCase", tag = "kind", content = "value")]
	#[try_unwrap(ref)]
	pub enum Outcome {
		Terminated,
		Canceled,
		Failed(Error),
		Succeeded(value::Data),
	}
}

impl Id {
	#[allow(clippy::new_without_default)]
	#[must_use]
	pub fn new() -> Self {
		Self(crate::Id::new_random(id::Kind::Build))
	}
}

impl Build {
	#[must_use]
	pub fn with_state(state: State) -> Self {
		Self {
			state: Arc::new(std::sync::RwLock::new(state)),
		}
	}

	#[must_use]
	pub fn state(&self) -> &std::sync::RwLock<State> {
		&self.state
	}

	#[must_use]
	pub fn with_id(id: Id) -> Self {
		let state = State::with_id(id);
		Self {
			state: Arc::new(std::sync::RwLock::new(state)),
		}
	}

	#[must_use]
	pub fn with_object(object: Object) -> Self {
		let state = State::with_object(object);
		Self {
			state: Arc::new(std::sync::RwLock::new(state)),
		}
	}

	#[must_use]
	pub fn id(&self) -> &Id {
		unsafe { &*(self.state.read().unwrap().id.as_ref().unwrap() as *const Id) }
	}

	#[must_use]
	pub fn try_get_loaded_object(&self) -> Option<&Object> {
		self.state
			.read()
			.unwrap()
			.object
			.as_ref()
			.map(|object| unsafe { &*(object as *const Object) })
	}

	pub async fn object(&self, client: &dyn Client) -> Result<&Object> {
		self.load(client).await?;
		Ok(unsafe { &*(self.state.read().unwrap().object.as_ref().unwrap() as *const Object) })
	}

	pub async fn try_get_object(&self, client: &dyn Client) -> Result<Option<&Object>> {
		if !self.try_load(client).await? {
			return Ok(None);
		}
		Ok(Some(unsafe {
			&*(self.state.read().unwrap().object.as_ref().unwrap() as *const Object)
		}))
	}

	pub async fn load(&self, client: &dyn Client) -> Result<()> {
		self.try_load(client)
			.await?
			.then_some(())
			.wrap_err(format!("Failed to load the object with id {}.", self.id()))
	}

	pub async fn try_load(&self, client: &dyn Client) -> Result<bool> {
		if self.state.read().unwrap().object.is_some() {
			return Ok(true);
		}
		let id = self.state.read().unwrap().id.clone().unwrap();
		let Some(bytes) = client.try_get_object(&id.clone().into()).await? else {
			return Ok(false);
		};
		let data = Data::deserialize(&bytes).wrap_err("Failed to deserialize the data.")?;
		let object = data.try_into()?;
		self.state.write().unwrap().object.replace(object);
		Ok(true)
	}

	#[async_recursion]
	pub async fn data(&self, client: &dyn Client) -> Result<Data> {
		let object = self.object(client).await?;
		let target = object.target.id(client).await?.clone();
		let children = object
			.children
			.iter()
			.map(|build| async { Ok::<_, Error>(build.id().clone()) })
			.collect::<FuturesOrdered<_>>()
			.try_collect()
			.await?;
		let log = object.log.id(client).await?;
		let outcome = match &object.outcome {
			Outcome::Terminated => data::Outcome::Terminated,
			Outcome::Canceled => data::Outcome::Canceled,
			Outcome::Failed(error) => data::Outcome::Failed(error.clone()),
			Outcome::Succeeded(value) => data::Outcome::Succeeded(value.data(client).await?),
		};
		Ok(Data {
			target,
			children,
			log,
			outcome,
		})
	}
}

impl Build {
	pub async fn new(
		client: &dyn Client,
		id: Id,
		target: Target,
		children: Vec<Build>,
		log: Blob,
		outcome: Outcome,
	) -> Result<Self> {
		let object = Object {
			target,
			children,
			log,
			outcome,
		};
		let build = Self::with_state(State {
			id: Some(id.clone()),
			object: Some(object),
		});
		let data = build.data(client).await?;
		let bytes = data.serialize()?;
		client
			.try_put_object(&id.clone().into(), &bytes)
			.await
			.wrap_err("Failed to put the object.")?
			.ok()
			.wrap_err("Expected the children to be stored.")?;
		Ok(build)
	}

	pub async fn target(&self, client: &dyn Client) -> Result<Target> {
		self.try_get_target(client)
			.await?
			.wrap_err("Failed to get the target.")
	}

	pub async fn try_get_target(&self, client: &dyn Client) -> Result<Option<Target>> {
		if let Some(object) = self.try_get_loaded_object() {
			return Ok(Some(object.target.clone()));
		}
		Ok(client
			.try_get_build_target(self.id())
			.await?
			.map(Target::with_id))
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
		if let Some(object) = self.try_get_loaded_object() {
			return Ok(Some(stream::iter(object.children.clone()).map(Ok).boxed()));
		}
		Ok(client
			.try_get_build_children(self.id())
			.await?
			.map(|children| children.map_ok(Build::with_id).boxed()))
	}

	pub async fn add_child(&self, client: &dyn Client, child: &Self) -> Result<()> {
		let id = self.id();
		let child_id = child.id();
		client.add_build_child(None, id, child_id).await?;
		Ok(())
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
		if let Some(object) = self.try_get_loaded_object() {
			let log = object.log.clone();
			let bytes = log.bytes(client).await?;
			return Ok(Some(stream::once(async move { Ok(bytes.into()) }).boxed()));
		}
		client.try_get_build_log(self.id()).await
	}

	pub async fn add_log(&self, client: &dyn Client, log: Bytes) -> Result<()> {
		let id = self.id();
		client.add_build_log(None, id, log).await?;
		Ok(())
	}

	pub async fn outcome(&self, client: &dyn Client) -> Result<Outcome> {
		self.try_get_outcome(client)
			.await?
			.wrap_err("Failed to get the build.")
	}

	pub async fn try_get_outcome(&self, client: &dyn Client) -> Result<Option<Outcome>> {
		if let Some(object) = self.try_get_loaded_object() {
			return Ok(Some(object.outcome.clone()));
		}
		client.try_get_build_outcome(self.id()).await
	}

	pub async fn cancel(&self, client: &dyn Client) -> Result<()> {
		let id = self.id();
		client.cancel_build(None, id).await?;
		Ok(())
	}

	pub async fn finish(
		&self,
		client: &dyn Client,
		user: Option<&User>,
		outcome: Outcome,
	) -> Result<()> {
		let id = self.id();
		client.finish_build(user, id, outcome).await?;
		Ok(())
	}
}

impl Outcome {
	#[must_use]
	pub fn retry(&self) -> Retry {
		match self {
			Self::Terminated => Retry::Terminated,
			Self::Canceled => Retry::Canceled,
			Self::Failed(_) => Retry::Failed,
			Self::Succeeded(_) => Retry::Succeeded,
		}
	}

	pub fn into_result(self) -> Result<Value> {
		match self {
			Self::Terminated => return_error!("The build was terminated."),
			Self::Canceled => return_error!("The build was canceled."),
			Self::Failed(error) => Err(error),
			Self::Succeeded(value) => Ok(value),
		}
	}

	pub async fn data(&self, client: &dyn Client) -> Result<data::Outcome> {
		Ok(match self {
			Self::Terminated => data::Outcome::Terminated,
			Self::Canceled => data::Outcome::Canceled,
			Self::Failed(error) => data::Outcome::Failed(error.clone()),
			Self::Succeeded(value) => data::Outcome::Succeeded(value.data(client).await?),
		})
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
		let outcome = self
			.outcome
			.try_unwrap_succeeded_ref()
			.ok()
			.map(value::Data::children)
			.into_iter()
			.flatten();
		std::iter::empty()
			.chain(target)
			.chain(children)
			.chain(log)
			.chain(outcome)
			.collect()
	}
}

impl TryFrom<Data> for Object {
	type Error = Error;

	fn try_from(data: Data) -> std::result::Result<Self, Self::Error> {
		let target = Target::with_id(data.target);
		let children = data.children.into_iter().map(Build::with_id).collect();
		let log = Blob::with_id(data.log);
		let outcome = data.outcome.try_into()?;
		Ok(Self {
			target,
			children,
			log,
			outcome,
		})
	}
}

impl TryFrom<data::Outcome> for Outcome {
	type Error = Error;

	fn try_from(data: data::Outcome) -> std::prelude::v1::Result<Self, Self::Error> {
		match data {
			data::Outcome::Terminated => Ok(Outcome::Terminated),
			data::Outcome::Canceled => Ok(Outcome::Canceled),
			data::Outcome::Failed(error) => Ok(Outcome::Failed(error)),
			data::Outcome::Succeeded(value) => Ok(Outcome::Succeeded(value.try_into()?)),
		}
	}
}

impl From<Id> for crate::Id {
	fn from(value: Id) -> Self {
		value.0
	}
}

impl TryFrom<crate::Id> for Id {
	type Error = Error;

	fn try_from(value: crate::Id) -> Result<Self, Self::Error> {
		if value.kind() != id::Kind::Build {
			return_error!("Invalid kind.");
		}
		Ok(Self(value))
	}
}

impl std::str::FromStr for Id {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		crate::Id::from_str(s)?.try_into()
	}
}

impl std::fmt::Display for Retry {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Terminated => write!(f, "terminated"),
			Self::Canceled => write!(f, "canceled"),
			Self::Failed => write!(f, "failed"),
			Self::Succeeded => write!(f, "succeeded"),
		}
	}
}

impl std::str::FromStr for Retry {
	type Err = Error;

	fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
		match s {
			"terminated" => Ok(Retry::Terminated),
			"canceled" => Ok(Retry::Canceled),
			"failed" => Ok(Retry::Failed),
			"succeeded" => Ok(Retry::Succeeded),
			_ => return_error!("Invalid retry."),
		}
	}
}

impl From<Retry> for String {
	fn from(value: Retry) -> Self {
		value.to_string()
	}
}

impl TryFrom<String> for Retry {
	type Error = Error;

	fn try_from(value: String) -> std::prelude::v1::Result<Self, Self::Error> {
		value.parse()
	}
}

pub mod queue {
	use super::{Id, Retry};

	#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
	pub struct Item {
		pub build: Id,
		pub depth: u64,
		pub retry: Retry,
	}
	impl PartialOrd for Item {
		fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
			Some(self.depth.cmp(&other.depth))
		}
	}

	impl Ord for Item {
		fn cmp(&self, other: &Self) -> std::cmp::Ordering {
			self.depth.cmp(&other.depth)
		}
	}

	impl PartialEq for Item {
		fn eq(&self, other: &Self) -> bool {
			self.depth == other.depth
		}
	}

	impl Eq for Item {}
}
