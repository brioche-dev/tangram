use crate::{id, object, template, Artifact, Client, Result, Template};

crate::id!(Symlink);
crate::handle!(Symlink);
crate::data!();

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
