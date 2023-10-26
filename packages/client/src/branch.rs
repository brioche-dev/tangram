use crate::{blob, object, return_error, Blob, Client, Result, WrapErr};

crate::id!(Branch);
crate::handle!(Branch);

#[derive(Clone, Debug)]
pub struct Id(crate::Id);

#[derive(Clone, Debug)]
pub struct Branch(object::Handle);

#[derive(Clone, Debug)]
pub struct Object {
	pub children: Vec<(Blob, u64)>,
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
	pub children: Vec<(blob::Id, u64)>,
}

impl Branch {
	#[must_use]
	pub fn new(children: Vec<(Blob, u64)>) -> Self {
		Self(object::Handle::with_object(Object { children }.into()))
	}

	pub async fn children(&self, client: &dyn Client) -> Result<&Vec<(Blob, u64)>> {
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
			.map(|(blob, size)| (blob.expect_id().clone(), *size))
			.collect();
		Data { children }
	}

	#[allow(clippy::needless_pass_by_value)]
	#[must_use]
	pub fn from_data(data: Data) -> Self {
		let children = data
			.children
			.iter()
			.map(|(id, size)| (Blob::with_id(id.clone()), *size))
			.collect();
		Self { children }
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Handle> {
		self.children
			.iter()
			.map(|(blob, _)| blob.handle().clone())
			.collect()
	}
}

impl Data {
	pub fn serialize(&self) -> Result<Vec<u8>> {
		let mut bytes = Vec::new();
		byteorder::WriteBytesExt::write_u8(&mut bytes, 0)
			.wrap_err("Failed to write the version.")?;
		tangram_serialize::to_writer(self, &mut bytes).wrap_err("Failed to write the data.")?;
		Ok(bytes)
	}

	pub fn deserialize(mut bytes: &[u8]) -> Result<Self> {
		let version =
			byteorder::ReadBytesExt::read_u8(&mut bytes).wrap_err("Failed to read the version.")?;
		if version != 0 {
			return_error!(r#"Cannot deserialize with version "{version}"."#);
		}
		let value = tangram_serialize::from_reader(bytes).wrap_err("Failed to read the data.")?;
		Ok(value)
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Id> {
		self.children
			.iter()
			.map(|(id, _)| id.clone().into())
			.collect()
	}
}
