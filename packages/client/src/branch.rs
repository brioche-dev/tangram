pub use self::child::Child;
use crate::{blob, object, Blob, Client, Result, WrapErr};
use bytes::Bytes;

crate::id!(Branch);
crate::handle!(Branch);

#[derive(Clone, Debug)]
pub struct Id(crate::Id);

#[derive(Clone, Debug)]
pub struct Branch(object::Handle);

#[derive(Clone, Debug)]
pub struct Object {
	pub children: Vec<Child>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Data {
	pub children: Vec<child::Data>,
}

impl Branch {
	#[must_use]
	pub fn new(children: Vec<Child>) -> Self {
		Self(object::Handle::with_object(Object { children }.into()))
	}

	pub async fn children(&self, client: &dyn Client) -> Result<&Vec<Child>> {
		let object = self.object(client).await?;
		Ok(&object.children)
	}
}

impl Object {
	#[must_use]
	pub fn to_data(&self) -> Data {
		let children = self
			.children
			.iter()
			.map(|child| child::Data {
				blob: child.blob.expect_id().clone(),
				size: child.size,
			})
			.collect();
		Data { children }
	}

	#[allow(clippy::needless_pass_by_value)]
	#[must_use]
	pub fn from_data(data: Data) -> Self {
		let children = data
			.children
			.into_iter()
			.map(|child| Child {
				blob: Blob::with_id(child.blob),
				size: child.size,
			})
			.collect();
		Self { children }
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Handle> {
		self.children
			.iter()
			.map(|child| child.blob.handle().clone())
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
		self.children
			.iter()
			.map(|child| child.blob.clone().into())
			.collect()
	}
}

impl std::fmt::Display for Branch {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.expect_id())?;
		Ok(())
	}
}

pub mod child {
	use super::{blob, Blob};

	#[derive(Clone, Debug)]
	pub struct Child {
		pub blob: Blob,
		pub size: u64,
	}

	#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
	pub struct Data {
		pub blob: blob::Id,
		pub size: u64,
	}
}
