use crate::{
	branch, build, directory, file, id, leaf, package, return_error, symlink, target, Branch,
	Build, Client, Directory, Error, File, Leaf, Package, Result, Symlink, Target,
};
use bytes::Bytes;
use derive_more::{From, TryInto, TryUnwrap};

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

/// An object.
#[derive(Clone, Debug, From, TryInto, TryUnwrap)]
#[try_unwrap(ref)]
pub enum Handle {
	Leaf(Leaf),
	Branch(Branch),
	Directory(Directory),
	File(File),
	Symlink(Symlink),
	Package(Package),
	Target(Target),
	Build(Build),
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

#[derive(Debug)]
pub struct State<I, O> {
	pub id: Option<I>,
	pub object: Option<O>,
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
	pub fn with_id(id: Id) -> Self {
		match id {
			Id::Leaf(id) => Self::Leaf(Leaf::with_id(id)),
			Id::Branch(id) => Self::Branch(Branch::with_id(id)),
			Id::Directory(id) => Self::Directory(Directory::with_id(id)),
			Id::File(id) => Self::File(File::with_id(id)),
			Id::Symlink(id) => Self::Symlink(Symlink::with_id(id)),
			Id::Package(id) => Self::Package(Package::with_id(id)),
			Id::Target(id) => Self::Target(Target::with_id(id)),
			Id::Build(id) => Self::Build(Build::with_id(id)),
		}
	}

	#[must_use]
	pub fn with_object(object: Object) -> Self {
		match object {
			Object::Leaf(object) => Self::Leaf(Leaf::with_object(object)),
			Object::Branch(object) => Self::Branch(Branch::with_object(object)),
			Object::Directory(object) => Self::Directory(Directory::with_object(object)),
			Object::File(object) => Self::File(File::with_object(object)),
			Object::Symlink(object) => Self::Symlink(Symlink::with_object(object)),
			Object::Package(object) => Self::Package(Package::with_object(object)),
			Object::Target(object) => Self::Target(Target::with_object(object)),
			Object::Build(object) => Self::Build(Build::with_object(object)),
		}
	}

	pub async fn id(&self, client: &dyn Client) -> Result<Id> {
		match self {
			Self::Leaf(object) => object.id(client).await.cloned().map(Id::Leaf),
			Self::Branch(object) => object.id(client).await.cloned().map(Id::Branch),
			Self::Directory(object) => object.id(client).await.cloned().map(Id::Directory),
			Self::File(object) => object.id(client).await.cloned().map(Id::File),
			Self::Symlink(object) => object.id(client).await.cloned().map(Id::Symlink),
			Self::Package(object) => object.id(client).await.cloned().map(Id::Package),
			Self::Target(object) => object.id(client).await.cloned().map(Id::Target),
			Self::Build(object) => object.id(client).await.cloned().map(Id::Build),
		}
	}

	pub async fn object(&self, client: &dyn Client) -> Result<Object> {
		match self {
			Self::Leaf(object) => object.object(client).await.cloned().map(Object::Leaf),
			Self::Branch(object) => object.object(client).await.cloned().map(Object::Branch),
			Self::Directory(object) => object.object(client).await.cloned().map(Object::Directory),
			Self::File(object) => object.object(client).await.cloned().map(Object::File),
			Self::Symlink(object) => object.object(client).await.cloned().map(Object::Symlink),
			Self::Package(object) => object.object(client).await.cloned().map(Object::Package),
			Self::Target(object) => object.object(client).await.cloned().map(Object::Target),
			Self::Build(object) => object.object(client).await.cloned().map(Object::Build),
		}
	}
}

impl Data {
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
}

impl TryFrom<Data> for Object {
	type Error = Error;

	fn try_from(data: Data) -> std::result::Result<Self, Self::Error> {
		Ok(match data {
			Data::Leaf(data) => Self::Leaf(data.try_into()?),
			Data::Branch(data) => Self::Branch(data.try_into()?),
			Data::Directory(data) => Self::Directory(data.try_into()?),
			Data::File(data) => Self::File(data.try_into()?),
			Data::Symlink(data) => Self::Symlink(data.try_into()?),
			Data::Package(data) => Self::Package(data.try_into()?),
			Data::Target(data) => Self::Target(data.try_into()?),
			Data::Build(data) => Self::Build(data.try_into()?),
		})
	}
}

impl<I, O> State<I, O> {
	#[must_use]
	pub fn new(id: Option<I>, object: Option<O>) -> Self {
		assert!(id.is_some() || object.is_some());
		Self { id, object }
	}

	#[must_use]
	pub fn with_id(id: I) -> Self {
		Self {
			id: Some(id),
			object: None,
		}
	}

	#[must_use]
	pub fn with_object(object: O) -> Self {
		Self {
			id: None,
			object: Some(object),
		}
	}

	#[must_use]
	pub fn id(&self) -> &Option<I> {
		&self.id
	}

	#[must_use]
	pub fn object(&self) -> &Option<O> {
		&self.object
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
			_ => return_error!("Expected an object ID."),
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
