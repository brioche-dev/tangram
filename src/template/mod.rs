pub use self::component::Value as Component;
use crate::artifact;

pub mod component;
mod render;
mod unrender;

crate::id!();

crate::kind!(Template);

#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

#[derive(Clone, Debug)]
pub struct Value {
	pub components: Vec<component::Value>,
}

#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
pub struct Data {
	#[tangram_serialize(id = 0)]
	pub components: Vec<component::Data>,
}

impl Value {
	#[must_use]
	pub fn from_data(data: Data) -> Self {
		let components = data
			.components
			.into_iter()
			.map(|component| match component {
				component::Data::String(data) => component::Value::String(data),
				component::Data::Artifact(data) => {
					component::Value::Artifact(artifact::Handle::with_id(data))
				},
				component::Data::Placeholder(data) => {
					component::Value::Placeholder(crate::placeholder::Value { name: data.name })
				},
			})
			.collect();
		Value { components }
	}

	#[must_use]
	pub fn to_data(&self) -> Data {
		todo!()
	}

	#[must_use]
	pub fn empty() -> Self {
		Self { components: vec![] }
	}

	#[must_use]
	pub fn components(&self) -> &[component::Value] {
		&self.components
	}

	pub fn artifacts(&self) -> impl Iterator<Item = &artifact::Handle> {
		self.components
			.iter()
			.filter_map(|component| match component {
				component::Value::Artifact(artifact) => Some(artifact),
				_ => None,
			})
	}
}

impl Value {
	#[must_use]
	pub fn children(&self) -> Vec<crate::Handle> {
		self.components
			.iter()
			.filter_map(|component| match component {
				component::Value::Artifact(artifact) => Some(artifact.clone().into()),
				_ => None,
			})
			.collect()
	}
}

impl Data {
	#[must_use]
	pub fn children(&self) -> Vec<crate::Id> {
		self.components
			.iter()
			.filter_map(|component| match component {
				component::Data::Artifact(id) => Some((*id).into()),
				_ => None,
			})
			.collect()
	}
}

impl From<Vec<component::Value>> for Value {
	fn from(value: Vec<component::Value>) -> Self {
		Value { components: value }
	}
}

impl From<component::Value> for Value {
	fn from(value: component::Value) -> Self {
		vec![value].into()
	}
}

impl From<String> for Value {
	fn from(value: String) -> Self {
		vec![component::Value::String(value)].into()
	}
}

impl From<&str> for Value {
	fn from(value: &str) -> Self {
		value.to_owned().into()
	}
}

impl FromIterator<component::Value> for Value {
	fn from_iter<I: IntoIterator<Item = component::Value>>(value: I) -> Self {
		Value {
			components: value.into_iter().collect(),
		}
	}
}
