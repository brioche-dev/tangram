use crate::{object, return_error, template, Artifact, Client, Result, Template};

crate::id!(Symlink);
crate::handle!(Symlink);

#[derive(Clone, Copy, Debug)]
pub struct Id(crate::Id);

#[derive(Clone, Debug)]
pub struct Symlink(object::Handle);

#[derive(Clone, Debug)]
pub struct Object {
	pub target: Template,
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
	pub target: template::Data,
}

impl Symlink {
	#[must_use]
	pub fn new(target: Template) -> Self {
		Self::with_object(Object { target })
	}

	pub async fn target(&self, client: &dyn Client) -> Result<Template> {
		Ok(self.object(client).await?.target.clone())
	}

	pub async fn resolve(&self, client: &dyn Client) -> Result<Option<Artifact>> {
		self.resolve_from(client, None).await
	}

	#[allow(clippy::unused_async)]
	pub async fn resolve_from(
		&self,
		_client: &dyn Client,
		_from: Option<Self>,
	) -> Result<Option<Artifact>> {
		unimplemented!()
	}
}

impl Object {
	#[must_use]
	pub(crate) fn to_data(&self) -> Data {
		let target = self.target.to_data();
		Data { target }
	}

	#[allow(clippy::needless_pass_by_value)]
	#[must_use]
	pub(crate) fn from_data(data: Data) -> Self {
		let target = Template::from_data(data.target);
		Self { target }
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Handle> {
		self.target.children()
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
		self.target.children()
	}
}
