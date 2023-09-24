use crate::{id, object, template, Artifact, Client, Result, Template};

#[derive(Clone, Debug)]
pub struct Symlink(Handle);

crate::object!(Symlink);

#[derive(Clone, Debug)]
pub(crate) struct Object {
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
pub(crate) struct Data {
	#[tangram_serialize(id = 0)]
	pub target: template::Data,
}

impl Symlink {
	#[must_use]
	pub fn handle(&self) -> &Handle {
		&self.0
	}

	#[must_use]
	pub fn with_id(id: Id) -> Self {
		Self(Handle::with_id(id))
	}

	#[must_use]
	pub fn new(target: Template) -> Self {
		Self(Handle::with_object(Object { target }))
	}

	pub async fn target(&self, client: &Client) -> Result<Template> {
		Ok(self.0.object(client).await?.target.clone())
	}

	pub async fn resolve(&self, client: &Client) -> Result<Option<Artifact>> {
		self.resolve_from(client, None).await
	}

	#[allow(clippy::unused_async)]
	pub async fn resolve_from(
		&self,
		_client: &Client,
		_from: Option<Artifact>,
	) -> Result<Option<Artifact>> {
		unimplemented!()
	}
}

impl Id {
	#[must_use]
	pub fn with_data_bytes(bytes: &[u8]) -> Self {
		Self(crate::Id::new_hashed(id::Kind::Symlink, bytes))
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
	#[must_use]
	pub fn children(&self) -> Vec<object::Id> {
		self.target.children()
	}
}
