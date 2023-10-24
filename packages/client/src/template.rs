pub use self::component::Component;
use crate::{object, Artifact, Result};
use futures::{stream::FuturesOrdered, Future, TryStreamExt};
use itertools::Itertools;
use std::{borrow::Cow, path::PathBuf};

#[derive(Clone, Debug)]
pub struct Template {
	pub components: Vec<Component>,
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
	pub components: Vec<component::Data>,
}

impl Template {
	#[must_use]
	pub fn empty() -> Self {
		Self {
			components: Vec::new(),
		}
	}

	#[must_use]
	pub fn components(&self) -> &[Component] {
		&self.components
	}

	pub fn artifacts(&self) -> impl Iterator<Item = &Artifact> {
		self.components
			.iter()
			.filter_map(|component| match component {
				Component::String(_) => None,
				Component::Artifact(artifact) => Some(artifact),
			})
	}

	pub fn try_render_sync<'a, F>(&'a self, mut f: F) -> Result<String>
	where
		F: (FnMut(&'a Component) -> Result<Cow<'a, str>>) + 'a,
	{
		let mut string = String::new();
		for component in &self.components {
			string.push_str(&f(component)?);
		}
		Ok(string)
	}

	pub async fn try_render<'a, F, Fut>(&'a self, f: F) -> Result<String>
	where
		F: (FnMut(&'a Component) -> Fut) + 'a,
		Fut: Future<Output = Result<String>> + 'a,
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

	pub fn unrender(artifacts_paths: &[PathBuf], string: &str) -> Result<Self> {
		// Create the regex.
		let artifacts_paths = artifacts_paths
			.iter()
			.map(|artifacts_path| artifacts_path.to_str().unwrap())
			.join("|");
		let regex = format!(r"(?:{artifacts_paths})/([0-9a-f]{{64}})");
		let regex = regex::Regex::new(&regex).unwrap();

		let mut i = 0;
		let mut components = Vec::new();
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
			components.push(Component::Artifact(Artifact::with_id(id)));

			// Advance the cursor to the end of the match.
			i = match_.end();
		}

		// Add the remaining text as a string component.
		if i < string.len() {
			components.push(Component::String(string[i..].to_owned()));
		}

		// Create the template.
		Ok(Self { components })
	}

	pub fn to_data(&self) -> Data {
		let components = self
			.components
			.iter()
			.map(Component::to_data)
			.collect::<Vec<_>>();
		Data { components }
	}

	pub fn from_data(data: Data) -> Self {
		let components = data
			.components
			.into_iter()
			.map(Component::from_data)
			.collect();
		Self { components }
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Handle> {
		self.components
			.iter()
			.filter_map(|component| match component {
				Component::String(_) => None,
				Component::Artifact(artifact) => Some(artifact.handle().clone()),
			})
			.collect()
	}
}

impl Data {
	#[must_use]
	pub fn children(&self) -> Vec<object::Id> {
		self.components
			.iter()
			.filter_map(|component| match component {
				component::Data::String(_) => None,
				component::Data::Artifact(id) => Some(id.clone().into()),
			})
			.collect()
	}
}

impl From<Component> for Template {
	fn from(value: Component) -> Self {
		vec![value].into()
	}
}

impl From<Vec<Component>> for Template {
	fn from(value: Vec<Component>) -> Self {
		Self { components: value }
	}
}

impl FromIterator<Component> for Template {
	fn from_iter<I: IntoIterator<Item = Component>>(value: I) -> Self {
		Self {
			components: value.into_iter().collect(),
		}
	}
}

impl From<String> for Template {
	fn from(value: String) -> Self {
		vec![Component::String(value)].into()
	}
}

impl From<&str> for Template {
	fn from(value: &str) -> Self {
		value.to_owned().into()
	}
}

pub mod component {
	use crate::{artifact, Artifact};
	use derive_more::From;

	#[derive(Clone, Debug, From)]
	pub enum Component {
		String(String),
		Artifact(Artifact),
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
		String(String),
		#[tangram_serialize(id = 1)]
		Artifact(artifact::Id),
	}

	impl Component {
		#[must_use]
		pub fn to_data(&self) -> Data {
			match self {
				Self::String(string) => Data::String(string.clone()),
				Self::Artifact(artifact) => Data::Artifact(artifact.expect_id()),
			}
		}

		#[must_use]
		pub fn from_data(data: Data) -> Self {
			match data {
				Data::String(string) => Self::String(string),
				Data::Artifact(id) => Self::Artifact(Artifact::with_id(id)),
			}
		}
	}
}
