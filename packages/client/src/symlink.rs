use crate::{object, template, Artifact, Client, Result, Template, WrapErr};
use bytes::Bytes;

crate::id!(Symlink);
crate::handle!(Symlink);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Id(crate::Id);

#[derive(Clone, Debug)]
pub struct Symlink(object::Handle);

#[derive(Clone, Debug)]
pub struct Object {
	pub target: Template,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Data {
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
	pub fn to_data(&self) -> Data {
		let target = self.target.to_data();
		Data { target }
	}

	#[allow(clippy::needless_pass_by_value)]
	#[must_use]
	pub fn from_data(data: Data) -> Self {
		let target = Template::from_data(data.target);
		Self { target }
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Handle> {
		self.target.children()
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
		self.target.children()
	}
}

impl std::fmt::Display for Symlink {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.expect_id())?;
		Ok(())
	}
}
