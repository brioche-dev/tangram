pub use self::component::Value as Component;
use crate::{artifact, value, Result};
use futures::{stream::FuturesOrdered, TryStreamExt};
use itertools::Itertools;
use std::path::PathBuf;
use std::{borrow::Cow, future::Future};

crate::id!(Template);

#[derive(Clone, Debug)]
pub struct Handle(value::Handle);

crate::handle!(Template);

#[derive(Clone, Debug)]
pub struct Value {
	pub components: Vec<component::Value>,
}

crate::value!(Template);

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
	pub components: Vec<component::Data>,
}

impl Handle {
	pub fn unrender(artifacts_paths: &[PathBuf], string: &str) -> Result<Self> {
		// Create the regex.
		let artifacts_paths = artifacts_paths
			.iter()
			.map(|artifacts_path| artifacts_path.to_str().unwrap())
			.join("|");
		let regex = format!(r"(?:{artifacts_paths})/([0-9a-f]{{64}})");
		let regex = regex::Regex::new(&regex).unwrap();

		let mut i = 0;
		let mut components = vec![];
		for captures in regex.captures_iter(string) {
			// Add the text leading up to the capture as a string component.
			let match_ = captures.get(0).unwrap();
			if match_.start() > i {
				components.push(Component::String(string[i..match_.start()].to_owned()));
			}

			// Get and parse the ID.
			let id = captures.get(1).unwrap();
			let id = id.as_str().parse().unwrap();

			// Add an artifact component.
			components.push(Component::Artifact(value::Handle::with_id(id).try_into()?));

			// Advance the cursor to the end of the match.
			i = match_.end();
		}

		// Add the remaining text as a string component.
		if i < string.len() {
			components.push(Component::String(string[i..].to_owned()));
		}

		// Create the template.
		Ok(Self::with_value(components.into()))
	}
}

impl Value {
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

	pub fn try_render_sync<'a, F>(&'a self, mut f: F) -> Result<String>
	where
		F: (FnMut(&'a component::Value) -> Result<Cow<'a, str>>) + 'a,
	{
		let mut string = String::new();
		for component in &self.components {
			string.push_str(&f(component)?);
		}
		Ok(string)
	}

	pub async fn try_render<'a, F, Fut>(&'a self, f: F) -> Result<String>
	where
		F: FnMut(&'a component::Value) -> Fut,
		Fut: Future<Output = Result<Cow<'a, str>>>,
	{
		Ok(self
			.components
			.iter()
			.map(f)
			.collect::<FuturesOrdered<_>>()
			.try_collect::<Vec<_>>()
			.await?
			.join(""))
	}

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
		Self { components }
	}

	#[must_use]
	pub fn to_data(&self) -> Data {
		let components = self
			.components
			.iter()
			.map(|component| match component {
				component::Value::String(value) => component::Data::String(value.clone()),
				component::Value::Artifact(value) => component::Data::Artifact(value.expect_id()),
				component::Value::Placeholder(value) => {
					component::Data::Placeholder(crate::placeholder::Data {
						name: value.name.clone(),
					})
				},
			})
			.collect();
		Data { components }
	}

	#[must_use]
	pub fn children(&self) -> Vec<value::Handle> {
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

impl From<Vec<component::Value>> for Handle {
	fn from(value: Vec<component::Value>) -> Self {
		Self::with_value(value.into())
	}
}

impl From<component::Value> for Handle {
	fn from(value: component::Value) -> Self {
		Self::with_value(value.into())
	}
}

impl From<String> for Handle {
	fn from(value: String) -> Self {
		Self::with_value(value.into())
	}
}

impl From<&str> for Handle {
	fn from(value: &str) -> Self {
		Self::with_value(value.into())
	}
}

impl FromIterator<component::Value> for Handle {
	fn from_iter<I: IntoIterator<Item = component::Value>>(value: I) -> Self {
		Self::with_value(Value::from_iter(value))
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

pub mod component {
	use crate::{artifact, placeholder};

	#[derive(Clone, Debug)]
	pub enum Value {
		String(String),
		Artifact(artifact::Handle),
		Placeholder(placeholder::Value),
	}

	#[derive(
		Clone,
		Debug,
		serde::Deserialize,
		serde::Serialize,
		tangram_serialize::Deserialize,
		tangram_serialize::Serialize,
	)]
	#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
	pub enum Data {
		#[tangram_serialize(id = 0)]
		String(crate::string::Data),
		#[tangram_serialize(id = 1)]
		Artifact(artifact::Id),
		#[tangram_serialize(id = 2)]
		Placeholder(crate::placeholder::Data),
	}

	impl Value {
		#[must_use]
		pub fn as_string(&self) -> Option<&str> {
			if let Self::String(string) = self {
				Some(string)
			} else {
				None
			}
		}

		#[must_use]
		pub fn as_artifact(&self) -> Option<&artifact::Handle> {
			if let Self::Artifact(artifact) = self {
				Some(artifact)
			} else {
				None
			}
		}

		#[must_use]
		pub fn as_placeholder(&self) -> Option<&placeholder::Value> {
			if let Self::Placeholder(placeholder) = self {
				Some(placeholder)
			} else {
				None
			}
		}

		#[must_use]
		pub fn into_string(self) -> Option<String> {
			if let Self::String(string) = self {
				Some(string)
			} else {
				None
			}
		}

		#[must_use]
		pub fn into_artifact(self) -> Option<artifact::Handle> {
			if let Self::Artifact(artifact) = self {
				Some(artifact)
			} else {
				None
			}
		}

		#[must_use]
		pub fn into_placeholder(self) -> Option<placeholder::Value> {
			if let Self::Placeholder(placeholder) = self {
				Some(placeholder)
			} else {
				None
			}
		}
	}
}
