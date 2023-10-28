use crate::{
	branch, build, directory, file, id, leaf, package, return_error, symlink, target, Client,
	Error, Result, WrapErr,
};
use bytes::Bytes;
use derive_more::{From, TryInto, TryUnwrap};
use futures::stream::TryStreamExt;
use std::sync::Arc;

/// An artifact kind.
#[derive(Clone, Copy, Debug)]
pub enum Kind {
	Leaf,
	Branch,
	Directory,
	File,
	Symlink,
	Package,
	Target,
	Build,
}

/// An object ID.
#[derive(Clone, Debug, From, TryInto, TryUnwrap, serde::Deserialize, serde::Serialize)]
#[serde(into = "crate::Id", try_from = "crate::Id")]
pub enum Id {
	Leaf(leaf::Id),
	Branch(branch::Id),
	Directory(directory::Id),
	File(file::Id),
	Symlink(symlink::Id),
	Package(package::Id),
	Target(target::Id),
	Build(build::Id),
}

/// An object handle.
#[derive(Clone, Debug)]
pub struct Handle {
	state: Arc<std::sync::RwLock<State>>,
}

#[derive(Debug)]
pub struct State {
	id: Option<Id>,
	object: Option<Object>,
}

/// An object.
#[derive(Clone, Debug, From, TryInto, TryUnwrap)]
#[try_unwrap(ref)]
pub enum Object {
	Leaf(leaf::Object),
	Branch(branch::Object),
	Directory(directory::Object),
	File(file::Object),
	Symlink(symlink::Object),
	Package(package::Object),
	Target(target::Object),
	Build(build::Object),
}

/// Object data.
#[derive(Clone, Debug, From, TryInto)]
pub enum Data {
	Leaf(leaf::Data),
	Branch(branch::Data),
	Directory(directory::Data),
	File(file::Data),
	Symlink(symlink::Data),
	Package(package::Data),
	Target(target::Data),
	Build(build::Data),
}

impl Id {
	#[must_use]
	pub fn new(kind: Kind, bytes: &[u8]) -> Self {
		match kind {
			Kind::Leaf => Self::Leaf(
				crate::Id::new_hashed(id::Kind::Leaf, bytes)
					.try_into()
					.unwrap(),
			),
			Kind::Branch => Self::Branch(
				crate::Id::new_hashed(id::Kind::Branch, bytes)
					.try_into()
					.unwrap(),
			),
			Kind::Directory => Self::Directory(
				crate::Id::new_hashed(id::Kind::Directory, bytes)
					.try_into()
					.unwrap(),
			),
			Kind::File => Self::File(
				crate::Id::new_hashed(id::Kind::File, bytes)
					.try_into()
					.unwrap(),
			),
			Kind::Symlink => Self::Symlink(
				crate::Id::new_hashed(id::Kind::Symlink, bytes)
					.try_into()
					.unwrap(),
			),
			Kind::Package => Self::Package(
				crate::Id::new_hashed(id::Kind::Package, bytes)
					.try_into()
					.unwrap(),
			),
			Kind::Target => Self::Target(
				crate::Id::new_hashed(id::Kind::Target, bytes)
					.try_into()
					.unwrap(),
			),
			Kind::Build => Self::Build(crate::Id::new_random(id::Kind::Build).try_into().unwrap()),
		}
	}

	#[must_use]
	pub fn kind(&self) -> Kind {
		match self {
			Self::Leaf(_) => Kind::Leaf,
			Self::Branch(_) => Kind::Branch,
			Self::Directory(_) => Kind::Directory,
			Self::File(_) => Kind::File,
			Self::Symlink(_) => Kind::Symlink,
			Self::Package(_) => Kind::Package,
			Self::Target(_) => Kind::Target,
			Self::Build(_) => Kind::Build,
		}
	}

	#[must_use]
	pub fn to_bytes(&self) -> Bytes {
		match self {
			Self::Leaf(id) => id.to_bytes(),
			Self::Branch(id) => id.to_bytes(),
			Self::Directory(id) => id.to_bytes(),
			Self::File(id) => id.to_bytes(),
			Self::Symlink(id) => id.to_bytes(),
			Self::Package(id) => id.to_bytes(),
			Self::Target(id) => id.to_bytes(),
			Self::Build(id) => id.to_bytes(),
		}
	}
}

impl Handle {
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
	pub fn expect_id(&self) -> &Id {
		unsafe { &*(self.state.read().unwrap().id.as_ref().unwrap() as *const Id) }
	}

	#[must_use]
	pub fn expect_object(&self) -> &Object {
		unsafe { &*(self.state.read().unwrap().object.as_ref().unwrap() as *const Object) }
	}

	pub async fn id(&self, client: &dyn Client) -> Result<&Id> {
		// Store the object.
		self.store(client).await?;

		// Return a reference to the ID.
		Ok(unsafe { &*(self.state.read().unwrap().id.as_ref().unwrap() as *const Id) })
	}

	pub async fn object(&self, client: &dyn Client) -> Result<&Object> {
		// Load the object.
		self.load(client).await?;

		// Return a reference to the object.
		Ok(unsafe { &*(self.state.read().unwrap().object.as_ref().unwrap() as *const Object) })
	}

	pub async fn try_get_object(&self, client: &dyn Client) -> Result<Option<&Object>> {
		// Attempt to load the object.
		if !self.try_load(client).await? {
			return Ok(None);
		}

		// Return a reference to the object.
		Ok(Some(unsafe {
			&*(self.state.read().unwrap().object.as_ref().unwrap() as *const Object)
		}))
	}

	pub async fn data(&self, client: &dyn Client) -> Result<Data> {
		// Load the object.
		self.load(client).await?;

		// Return the data.
		Ok(self
			.state
			.read()
			.unwrap()
			.object
			.as_ref()
			.unwrap()
			.to_data())
	}

	pub async fn load(&self, client: &dyn Client) -> Result<()> {
		self.try_load(client)
			.await?
			.then_some(())
			.wrap_err("Failed to load the object.")
	}

	pub async fn try_load(&self, client: &dyn Client) -> Result<bool> {
		// If the handle is loaded, then return.
		if self.state.read().unwrap().object.is_some() {
			return Ok(true);
		}

		// Get the ID.
		let id = self.expect_id();

		// Get the kind.
		let kind = id.kind();

		// Get the data.
		let Some(bytes) = client.try_get_object_bytes(id).await? else {
			return Ok(false);
		};

		// Deserialize the data.
		let data = Data::deserialize(kind, &bytes).wrap_err("Failed to deserialize the data.")?;

		// Create the object.
		let object = Object::from_data(data);

		// Update the state.
		self.state.write().unwrap().object.replace(object);

		Ok(true)
	}

	#[async_recursion::async_recursion]
	pub async fn store(&self, client: &dyn Client) -> Result<()> {
		// If the handle is stored, then return.
		if self.state.read().unwrap().id.is_some() {
			return Ok(());
		}

		// Get the children.
		let children = self
			.state
			.read()
			.unwrap()
			.object
			.as_ref()
			.unwrap()
			.children();

		// Store the children.
		children
			.into_iter()
			.map(|child| async move { child.store(client).await })
			.collect::<futures::stream::FuturesUnordered<_>>()
			.try_collect()
			.await?;

		// Get the data.
		let data = self
			.state
			.read()
			.unwrap()
			.object
			.as_ref()
			.unwrap()
			.to_data();

		// Serialize the data.
		let bytes = data.serialize()?;

		// Get the kind.
		let kind = data.kind();

		// Create the ID.
		let id = Id::new(kind, &bytes);

		// Store the object.
		client
			.try_put_object_bytes(&id, &bytes)
			.await
			.wrap_err("Failed to put the object.")?
			.ok()
			.wrap_err("Expected all children to be stored.")?;

		// Update the state.
		self.state.write().unwrap().id.replace(id);

		Ok(())
	}
}

impl State {
	#[must_use]
	pub fn new(id: Option<Id>, object: Option<Object>) -> Self {
		assert!(id.is_some() || object.is_some());
		Self { id, object }
	}

	#[must_use]
	pub fn with_id(id: Id) -> Self {
		Self {
			id: Some(id),
			object: None,
		}
	}

	#[must_use]
	pub fn with_object(object: Object) -> Self {
		Self {
			id: None,
			object: Some(object),
		}
	}

	#[must_use]
	pub fn id(&self) -> &Option<Id> {
		&self.id
	}

	#[must_use]
	pub fn object(&self) -> &Option<Object> {
		&self.object
	}
}

impl Object {
	#[must_use]
	pub fn to_data(&self) -> Data {
		match self {
			Self::Leaf(leaf) => Data::Leaf(leaf.to_data()),
			Self::Branch(branch) => Data::Branch(branch.to_data()),
			Self::Directory(directory) => Data::Directory(directory.to_data()),
			Self::File(file) => Data::File(file.to_data()),
			Self::Symlink(symlink) => Data::Symlink(symlink.to_data()),
			Self::Package(package) => Data::Package(package.to_data()),
			Self::Target(target) => Data::Target(target.to_data()),
			Self::Build(build) => Data::Build(build.to_data()),
		}
	}

	#[must_use]
	pub fn from_data(data: Data) -> Self {
		match data {
			Data::Leaf(data) => Self::Leaf(leaf::Object::from_data(data)),
			Data::Branch(data) => Self::Branch(branch::Object::from_data(data)),
			Data::Directory(data) => Self::Directory(directory::Object::from_data(data)),
			Data::File(data) => Self::File(file::Object::from_data(data)),
			Data::Symlink(data) => Self::Symlink(symlink::Object::from_data(data)),
			Data::Package(data) => Self::Package(package::Object::from_data(data)),
			Data::Target(data) => Self::Target(target::Object::from_data(data)),
			Data::Build(data) => Self::Build(build::Object::from_data(data)),
		}
	}

	#[must_use]
	pub fn children(&self) -> Vec<Handle> {
		match self {
			Self::Leaf(leaf) => leaf.children(),
			Self::Branch(branch) => branch.children(),
			Self::Directory(directory) => directory.children(),
			Self::File(file) => file.children(),
			Self::Symlink(symlink) => symlink.children(),
			Self::Package(package) => package.children(),
			Self::Target(target) => target.children(),
			Self::Build(build) => build.children(),
		}
	}
}

impl Data {
	#[allow(dead_code)]
	pub fn serialize(&self) -> Result<Bytes> {
		match self {
			Self::Leaf(data) => Ok(data.serialize()?),
			Self::Branch(data) => Ok(data.serialize()?),
			Self::Directory(data) => Ok(data.serialize()?),
			Self::File(data) => Ok(data.serialize()?),
			Self::Symlink(data) => Ok(data.serialize()?),
			Self::Package(data) => Ok(data.serialize()?),
			Self::Target(data) => Ok(data.serialize()?),
			Self::Build(data) => Ok(data.serialize()?),
		}
	}

	pub fn deserialize(kind: Kind, bytes: &Bytes) -> Result<Self> {
		match kind {
			Kind::Leaf => Ok(Self::Leaf(leaf::Data::deserialize(bytes)?)),
			Kind::Branch => Ok(Self::Branch(branch::Data::deserialize(bytes)?)),
			Kind::Directory => Ok(Self::Directory(directory::Data::deserialize(bytes)?)),
			Kind::File => Ok(Self::File(file::Data::deserialize(bytes)?)),
			Kind::Symlink => Ok(Self::Symlink(symlink::Data::deserialize(bytes)?)),
			Kind::Package => Ok(Self::Package(package::Data::deserialize(bytes)?)),
			Kind::Target => Ok(Self::Target(target::Data::deserialize(bytes)?)),
			Kind::Build => Ok(Self::Build(build::Data::deserialize(bytes)?)),
		}
	}

	#[must_use]
	pub fn children(&self) -> Vec<self::Id> {
		match self {
			Self::Leaf(data) => data.children(),
			Self::Branch(data) => data.children(),
			Self::Directory(data) => data.children(),
			Self::File(data) => data.children(),
			Self::Symlink(data) => data.children(),
			Self::Package(data) => data.children(),
			Self::Target(data) => data.children(),
			Self::Build(data) => data.children(),
		}
	}

	#[must_use]
	pub fn kind(&self) -> Kind {
		match self {
			Self::Leaf(_) => Kind::Leaf,
			Self::Branch(_) => Kind::Branch,
			Self::Directory(_) => Kind::Directory,
			Self::File(_) => Kind::File,
			Self::Symlink(_) => Kind::Symlink,
			Self::Package(_) => Kind::Package,
			Self::Target(_) => Kind::Target,
			Self::Build(_) => Kind::Build,
		}
	}
}

impl From<self::Id> for crate::Id {
	fn from(value: self::Id) -> Self {
		match value {
			self::Id::Leaf(id) => id.into(),
			self::Id::Branch(id) => id.into(),
			self::Id::Directory(id) => id.into(),
			self::Id::File(id) => id.into(),
			self::Id::Symlink(id) => id.into(),
			self::Id::Package(id) => id.into(),
			self::Id::Target(id) => id.into(),
			self::Id::Build(id) => id.into(),
		}
	}
}

impl TryFrom<crate::Id> for self::Id {
	type Error = Error;

	fn try_from(value: crate::Id) -> Result<Self, Self::Error> {
		match value.kind() {
			crate::id::Kind::Leaf => Ok(Self::Leaf(value.try_into()?)),
			crate::id::Kind::Branch => Ok(Self::Branch(value.try_into()?)),
			crate::id::Kind::Directory => Ok(Self::Directory(value.try_into()?)),
			crate::id::Kind::File => Ok(Self::File(value.try_into()?)),
			crate::id::Kind::Symlink => Ok(Self::Symlink(value.try_into()?)),
			crate::id::Kind::Package => Ok(Self::Package(value.try_into()?)),
			crate::id::Kind::Target => Ok(Self::Target(value.try_into()?)),
			crate::id::Kind::Build => Ok(Self::Build(value.try_into()?)),
			_ => return_error!("Expected a valid object ID."),
		}
	}
}

impl std::fmt::Display for Id {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Leaf(id) => write!(f, "{id}"),
			Self::Branch(id) => write!(f, "{id}"),
			Self::Directory(id) => write!(f, "{id}"),
			Self::File(id) => write!(f, "{id}"),
			Self::Symlink(id) => write!(f, "{id}"),
			Self::Package(id) => write!(f, "{id}"),
			Self::Target(id) => write!(f, "{id}"),
			Self::Build(id) => write!(f, "{id}"),
		}
	}
}

impl std::str::FromStr for Id {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		crate::Id::from_str(s)?.try_into()
	}
}

#[macro_export]
macro_rules! handle {
	($t:ident) => {
		impl $t {
			#[must_use]
			pub fn with_handle(handle: $crate::object::Handle) -> Self {
				Self(handle)
			}

			#[must_use]
			pub fn with_id(id: Id) -> Self {
				Self($crate::object::Handle::with_id(id.into()))
			}

			#[must_use]
			pub fn with_object(object: Object) -> Self {
				Self($crate::object::Handle::with_object($crate::Object::$t(
					object,
				)))
			}

			#[must_use]
			pub fn expect_id(&self) -> &Id {
				match self.0.expect_id() {
					$crate::object::Id::$t(id) => id,
					_ => unreachable!(),
				}
			}

			#[must_use]
			pub fn expect_object(&self) -> &Object {
				match self.0.expect_object() {
					$crate::object::Object::$t(object) => object,
					_ => unreachable!(),
				}
			}

			pub async fn id(&self, client: &dyn $crate::Client) -> $crate::Result<&Id> {
				match self.0.id(client).await? {
					$crate::object::Id::$t(id) => Ok(id),
					_ => unreachable!(),
				}
			}

			pub async fn object(&self, client: &dyn $crate::Client) -> $crate::Result<&Object> {
				match self.0.object(client).await? {
					$crate::object::Object::$t(object) => Ok(object),
					_ => unreachable!(),
				}
			}

			pub async fn data(&self, client: &dyn $crate::Client) -> $crate::Result<Data> {
				match self.0.data(client).await? {
					$crate::object::Data::$t(data) => Ok(data),
					_ => unreachable!(),
				}
			}

			pub async fn load(&self, client: &dyn $crate::Client) -> $crate::Result<()> {
				self.0.load(client).await
			}

			pub async fn store(&self, client: &dyn $crate::Client) -> $crate::Result<()> {
				self.0.store(client).await
			}

			#[must_use]
			pub fn handle(&self) -> &$crate::object::Handle {
				&self.0
			}
		}
	};
}

#[macro_export]
macro_rules! id {
	($t:ident) => {
		impl self::Id {
			#[must_use]
			pub fn to_bytes(&self) -> ::bytes::Bytes {
				self.0.to_bytes()
			}
		}

		impl serde::Serialize for self::Id {
			fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
				self.0.serialize(serializer)
			}
		}

		impl<'de> serde::Deserialize<'de> for self::Id {
			fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
				$crate::Id::deserialize(deserializer)?
					.try_into()
					.map_err(serde::de::Error::custom)
			}
		}

		impl std::fmt::Display for self::Id {
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				write!(f, "{}", self.0)?;
				Ok(())
			}
		}

		impl std::str::FromStr for self::Id {
			type Err = $crate::Error;

			fn from_str(s: &str) -> Result<Self, Self::Err> {
				$crate::Id::from_str(s)?.try_into()
			}
		}

		impl TryFrom<&[u8]> for self::Id {
			type Error = $crate::Error;

			fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
				Self::try_from($crate::Id::try_from(value)?)
			}
		}

		impl TryFrom<Vec<u8>> for self::Id {
			type Error = $crate::Error;

			fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
				Self::try_from($crate::Id::try_from(value)?)
			}
		}

		impl From<self::Id> for $crate::Id {
			fn from(value: self::Id) -> Self {
				value.0
			}
		}

		impl TryFrom<$crate::Id> for self::Id {
			type Error = $crate::Error;

			fn try_from(value: $crate::Id) -> Result<Self, Self::Error> {
				match value.kind() {
					$crate::id::Kind::$t => Ok(Self(value)),
					_ => $crate::return_error!("Unexpected kind."),
				}
			}
		}
	};
}
