use crate::{
	artifact, blob, directory, error, file, package, return_error, symlink, task, Client, Error,
	Result, WrapErr,
};
use derive_more::{From, TryInto};
use futures::stream::TryStreamExt;

/// An artifact kind.
#[derive(Clone, Copy, Debug)]
pub enum Kind {
	Blob,
	Directory,
	File,
	Symlink,
	Package,
	Task,
	// Run,
}

/// An object ID.
#[derive(
	Clone,
	Copy,
	Debug,
	From,
	TryInto,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(into = "crate::Id", try_from = "crate::Id")]
#[tangram_serialize(into = "crate::Id", try_from = "crate::Id")]
pub enum Id {
	Blob(blob::Id),
	Directory(directory::Id),
	File(file::Id),
	Symlink(symlink::Id),
	Package(package::Id),
	Task(task::Id),
	// Run(run::Id),
}

/// An object handle.
#[derive(Clone, Debug)]
pub struct Handle {
	id: std::sync::Arc<std::sync::RwLock<Option<Id>>>,
	object: std::sync::Arc<std::sync::RwLock<Option<Object>>>,
}

/// An object.
#[derive(Clone, Debug, From, TryInto)]
pub enum Object {
	Blob(blob::Object),
	Directory(directory::Object),
	File(file::Object),
	Symlink(symlink::Object),
	Package(package::Object),
	Task(task::Object),
	// Run(run::Object),
}

/// Object data.
#[derive(Clone, Debug, From, TryInto)]
pub(crate) enum Data {
	Blob(blob::Data),
	Directory(directory::Data),
	File(file::Data),
	Symlink(symlink::Data),
	Package(package::Data),
	Task(task::Data),
	// Run(run::Data),
}

impl Id {
	#[must_use]
	pub fn new(kind: Kind, bytes: &[u8]) -> Self {
		match kind {
			Kind::Blob => Self::Blob(blob::Id::new(bytes)),
			Kind::Directory => Self::Directory(directory::Id::new(bytes)),
			Kind::File => Self::File(file::Id::new(bytes)),
			Kind::Symlink => Self::Symlink(symlink::Id::new(bytes)),
			Kind::Package => Self::Package(package::Id::new(bytes)),
			Kind::Task => Self::Task(task::Id::new(bytes)),
			// Kind::Run => Self::Run(run::Id::new()),
		}
	}

	#[must_use]
	pub fn kind(&self) -> Kind {
		match self {
			Self::Blob(_) => Kind::Blob,
			Self::Directory(_) => Kind::Directory,
			Self::File(_) => Kind::File,
			Self::Symlink(_) => Kind::Symlink,
			Self::Package(_) => Kind::Package,
			Self::Task(_) => Kind::Task,
			// Self::Run(_) => Kind::Run,
		}
	}

	#[must_use]
	pub fn as_bytes(&self) -> [u8; crate::id::SIZE] {
		match self {
			Self::Blob(id) => id.as_bytes(),
			Self::Directory(id) => id.as_bytes(),
			Self::File(id) => id.as_bytes(),
			Self::Symlink(id) => id.as_bytes(),
			Self::Package(id) => id.as_bytes(),
			Self::Task(id) => id.as_bytes(),
			// Self::Run(id) => id.as_bytes(),
		}
	}
}

impl Handle {
	#[must_use]
	pub fn with_id(id: Id) -> Self {
		Self {
			id: std::sync::Arc::new(std::sync::RwLock::new(Some(id))),
			object: std::sync::Arc::new(std::sync::RwLock::new(None)),
		}
	}

	#[must_use]
	pub(crate) fn with_object(object: Object) -> Self {
		Self {
			id: std::sync::Arc::new(std::sync::RwLock::new(None)),
			object: std::sync::Arc::new(std::sync::RwLock::new(Some(object))),
		}
	}

	#[must_use]
	pub fn expect_id(&self) -> Id {
		self.id.read().unwrap().unwrap()
	}

	#[must_use]
	pub fn expect_object(&self) -> &Object {
		unsafe { &*(self.object.read().unwrap().as_ref().unwrap() as *const Object) }
	}

	pub async fn id(&self, client: &Client) -> Result<Id> {
		// Store the object.
		self.store(client).await?;

		// Return the ID.
		Ok(self.id.read().unwrap().unwrap())
	}

	pub async fn object(&self, client: &Client) -> Result<&Object> {
		// Load the object.
		self.load(client).await?;

		// Return a reference to the object.
		Ok(unsafe { &*(self.object.read().unwrap().as_ref().unwrap() as *const Object) })
	}

	pub async fn load(&self, client: &Client) -> Result<()> {
		// If the object is already loaded, then return.
		if self.object.read().unwrap().is_some() {
			return Ok(());
		}

		// Get the id.
		let id = self.id.read().unwrap().unwrap();

		// Get the kind.
		let kind = id.kind();

		// Get the data.
		let Some(bytes) = client.try_get_object_bytes(id).await? else {
			return_error!(r#"Failed to find object with id "{id}"."#);
		};

		// Deserialize the object.
		let data = Data::deserialize(kind, &bytes)?;

		// Create the object.
		let object = Object::from_data(data);

		// Set the object.
		self.object.write().unwrap().replace(object);

		Ok(())
	}

	#[async_recursion::async_recursion]
	pub async fn store(&self, client: &Client) -> Result<()> {
		// If the object is already stored, then return.
		if self.id.read().unwrap().is_some() {
			return Ok(());
		}

		// Create the data.
		let data = self.object.read().unwrap().as_ref().unwrap().to_data();

		// Store the children.
		let children = self.object.read().unwrap().as_ref().unwrap().children();
		children
			.into_iter()
			.map(|child| async move { child.store(client).await })
			.collect::<futures::stream::FuturesUnordered<_>>()
			.try_collect()
			.await?;

		// Serialize the data.
		let bytes = data.serialize()?;

		// Get the kind.
		let kind = data.kind();

		// Create the ID.
		let id = Id::new(kind, &bytes);

		// Store the object.
		client
			.try_put_object_bytes(id, &bytes)
			.await
			.wrap_err("Failed to put the object.")?
			.map_err(|_| error!("Expected all children to be stored."))?;

		// Set the ID.
		self.id.write().unwrap().replace(id);

		Ok(())
	}
}

impl Object {
	pub(crate) fn to_data(&self) -> Data {
		match self {
			Self::Blob(blob) => Data::Blob(blob.to_data()),
			Self::Directory(directory) => Data::Directory(directory.to_data()),
			Self::File(file) => Data::File(file.to_data()),
			Self::Symlink(symlink) => Data::Symlink(symlink.to_data()),
			Self::Package(package) => Data::Package(package.to_data()),
			Self::Task(task) => Data::Task(task.to_data()),
			// Self::Run(run) => Data::Run(run.to_data()),
		}
	}

	pub(crate) fn from_data(data: Data) -> Self {
		match data {
			Data::Blob(data) => Self::Blob(blob::Object::from_data(data)),
			Data::Directory(data) => Self::Directory(directory::Object::from_data(data)),
			Data::File(data) => Self::File(file::Object::from_data(data)),
			Data::Symlink(data) => Self::Symlink(symlink::Object::from_data(data)),
			Data::Package(data) => Self::Package(package::Object::from_data(data)),
			Data::Task(data) => Self::Task(task::Object::from_data(data)),
			// Data::Run(data) => Self::Run(run::Object::from_data(data)),
		}
	}

	#[must_use]
	pub fn children(&self) -> Vec<Handle> {
		match self {
			Self::Blob(blob) => blob.children(),
			Self::Directory(directory) => directory.children(),
			Self::File(file) => file.children(),
			Self::Symlink(symlink) => symlink.children(),
			Self::Package(package) => package.children(),
			Self::Task(task) => task.children(),
			// Self::Run(run) => run.children(),
		}
	}
}

impl Data {
	#[allow(unused)]
	pub fn serialize(&self) -> Result<Vec<u8>> {
		match self {
			Self::Blob(data) => Ok(data.serialize()?),
			Self::Directory(data) => Ok(data.serialize()?),
			Self::File(data) => Ok(data.serialize()?),
			Self::Symlink(data) => Ok(data.serialize()?),
			Self::Package(data) => Ok(data.serialize()?),
			Self::Task(data) => Ok(data.serialize()?),
			// Self::Run(data) => Ok(data.serialize()?),
		}
	}

	pub fn deserialize(kind: Kind, bytes: &[u8]) -> Result<Self> {
		match kind {
			Kind::Blob => Ok(Self::Blob(blob::Data::deserialize(bytes)?)),
			Kind::Directory => Ok(Self::Directory(directory::Data::deserialize(bytes)?)),
			Kind::File => Ok(Self::File(file::Data::deserialize(bytes)?)),
			Kind::Symlink => Ok(Self::Symlink(symlink::Data::deserialize(bytes)?)),
			Kind::Package => Ok(Self::Package(package::Data::deserialize(bytes)?)),
			Kind::Task => Ok(Self::Task(task::Data::deserialize(bytes)?)),
			// Kind::Run => Ok(Self::Run(run::Data::deserialize(bytes)?)),
		}
	}

	pub fn children(&self) -> Vec<self::Id> {
		match self {
			Self::Blob(data) => data.children(),
			Self::Directory(data) => data.children(),
			Self::File(data) => data.children(),
			Self::Symlink(data) => data.children(),
			Self::Package(data) => data.children(),
			Self::Task(data) => data.children(),
			// Self::Run(data) => data.children(),
		}
	}

	pub fn kind(&self) -> Kind {
		match self {
			Self::Blob(_) => Kind::Blob,
			Self::Directory(_) => Kind::Directory,
			Self::File(_) => Kind::File,
			Self::Symlink(_) => Kind::Symlink,
			Self::Package(_) => Kind::Package,
			Self::Task(_) => Kind::Task,
			// Self::Run(_) => Kind::Run,
		}
	}
}

impl From<self::Id> for crate::Id {
	fn from(value: self::Id) -> Self {
		match value {
			self::Id::Blob(id) => id.into(),
			self::Id::Directory(id) => id.into(),
			self::Id::File(id) => id.into(),
			self::Id::Symlink(id) => id.into(),
			self::Id::Package(id) => id.into(),
			self::Id::Task(id) => id.into(),
			// self::Id::Run(id) => id.into(),
		}
	}
}

impl TryFrom<crate::Id> for self::Id {
	type Error = Error;

	fn try_from(value: crate::Id) -> Result<Self, Self::Error> {
		match value.kind() {
			crate::id::Kind::Blob => Ok(Self::Blob(value.try_into()?)),
			crate::id::Kind::Directory => Ok(Self::Directory(value.try_into()?)),
			crate::id::Kind::File => Ok(Self::File(value.try_into()?)),
			crate::id::Kind::Symlink => Ok(Self::Symlink(value.try_into()?)),
			crate::id::Kind::Package => Ok(Self::Package(value.try_into()?)),
			crate::id::Kind::Task => Ok(Self::Task(value.try_into()?)),
			// crate::id::Kind::Run => Ok(Self::Run(value.try_into()?)),
			_ => return_error!("Unexpected kind."),
		}
	}
}

impl std::fmt::Display for Id {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Blob(id) => write!(f, "{id}"),
			Self::Directory(id) => write!(f, "{id}"),
			Self::File(id) => write!(f, "{id}"),
			Self::Symlink(id) => write!(f, "{id}"),
			Self::Package(id) => write!(f, "{id}"),
			Self::Task(id) => write!(f, "{id}"),
			// Self::Run(id) => write!(f, "{id}"),
		}
	}
}

impl std::str::FromStr for Id {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		crate::Id::from_str(s)?.try_into()
	}
}

impl From<artifact::Id> for Id {
	fn from(value: artifact::Id) -> Self {
		match value {
			artifact::Id::Directory(id) => Self::Directory(id),
			artifact::Id::File(id) => Self::File(id),
			artifact::Id::Symlink(id) => Self::Symlink(id),
		}
	}
}

#[macro_export]
macro_rules! object {
	($t:ident) => {
		#[derive(
			Clone,
			Copy,
			Debug,
			Eq,
			Ord,
			PartialEq,
			PartialOrd,
			serde::Deserialize,
			serde::Serialize,
			tangram_serialize::Deserialize,
			tangram_serialize::Serialize,
		)]
		#[tangram_serialize(into = "crate::Id", try_from = "crate::Id")]
		pub struct Id($crate::Id);

		impl self::Id {
			#[must_use]
			pub fn as_bytes(&self) -> [u8; $crate::id::SIZE] {
				self.0.as_bytes()
			}
		}

		impl self::Data {
			pub(crate) fn serialize(&self) -> $crate::Result<Vec<u8>> {
				let mut bytes = Vec::new();
				byteorder::WriteBytesExt::write_u8(&mut bytes, 0)?;
				tangram_serialize::to_writer(self, &mut bytes)?;
				Ok(bytes)
			}

			pub(crate) fn deserialize(mut bytes: &[u8]) -> $crate::Result<Self> {
				let version = byteorder::ReadBytesExt::read_u8(&mut bytes)?;
				if version != 0 {
					$crate::return_error!(
						r#"Cannot deserialize this object with version "{version}"."#
					);
				}
				let value = tangram_serialize::from_reader(bytes)?;
				Ok(value)
			}
		}

		impl std::hash::Hash for self::Id {
			fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
				std::hash::Hash::hash(&self.0, state);
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

		impl From<self::Id> for $crate::Id {
			fn from(object: self::Id) -> Self {
				object.0
			}
		}

		impl TryFrom<$crate::Id> for self::Id {
			type Error = $crate::Error;

			fn try_from(object: $crate::Id) -> Result<Self, Self::Error> {
				match object.kind() {
					$crate::id::Kind::$t => Ok(Self(object)),
					_ => $crate::return_error!("Unexpected kind."),
				}
			}
		}
	};
}
