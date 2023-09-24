use crate::{id, object, template, Artifact, Client, Result, Template};

#[derive(Clone, Debug)]
pub struct Symlink(object::Handle);

crate::object!(Symlink);

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
pub(crate) struct Data {
	#[tangram_serialize(id = 0)]
	pub target: template::Data,
}

impl Symlink {
	#[must_use]
	pub fn with_id(id: Id) -> Self {
		Self(object::Handle::with_id(id.into()))
	}

	#[must_use]
	pub fn with_object(object: Object) -> Self {
		Self(object::Handle::with_object(object::Object::Symlink(object)))
	}

	#[must_use]
	pub fn expect_id(&self) -> Id {
		match self.0.expect_id() {
			object::Id::Symlink(id) => id,
			_ => unreachable!(),
		}
	}

	#[must_use]
	pub fn expect_object(&self) -> &Object {
		match self.0.expect_object() {
			object::Object::Symlink(object) => object,
			_ => unreachable!(),
		}
	}

	pub async fn id(&self, client: &Client) -> Result<Id> {
		Ok(match self.0.id(client).await? {
			object::Id::Symlink(id) => id,
			_ => unreachable!(),
		})
	}

	pub async fn object(&self, client: &Client) -> Result<&Object> {
		Ok(match self.0.object(client).await? {
			object::Object::Symlink(object) => object,
			_ => unreachable!(),
		})
	}

	#[must_use]
	pub fn handle(&self) -> &object::Handle {
		&self.0
	}

	#[must_use]
	pub fn new(target: Template) -> Self {
		Self::with_object(Object { target })
	}

	pub async fn target(&self, client: &Client) -> Result<Template> {
		Ok(self.object(client).await?.target.clone())
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
	pub fn new(bytes: &[u8]) -> Self {
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
