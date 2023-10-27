use crate::{object, Client, Result};
use bytes::Bytes;

crate::id!(Leaf);
crate::handle!(Leaf);

#[derive(Clone, Debug)]
pub struct Id(crate::Id);

#[derive(Clone, Debug)]
pub struct Leaf(object::Handle);

#[derive(Clone, Debug)]
pub struct Object {
	pub bytes: Bytes,
}

#[derive(Clone, Debug)]
pub struct Data {
	pub bytes: Bytes,
}

impl Leaf {
	#[must_use]
	pub fn new(bytes: Bytes) -> Self {
		Self(object::Handle::with_object(Object { bytes }.into()))
	}

	#[must_use]
	pub fn empty() -> Self {
		Self::new(Bytes::new())
	}

	pub async fn bytes(&self, client: &dyn Client) -> Result<&Bytes> {
		let object = self.object(client).await?;
		Ok(&object.bytes)
	}
}

impl Object {
	#[must_use]
	pub fn to_data(&self) -> Data {
		Data {
			bytes: self.bytes.clone(),
		}
	}

	#[must_use]
	pub fn from_data(data: Data) -> Self {
		Self { bytes: data.bytes }
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Handle> {
		vec![]
	}
}

impl Data {
	pub fn serialize(&self) -> Result<Bytes> {
		Ok(self.bytes.clone())
	}

	pub fn deserialize(bytes: &Bytes) -> Result<Self> {
		Ok(Self {
			bytes: bytes.clone(),
		})
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Id> {
		vec![]
	}
}
