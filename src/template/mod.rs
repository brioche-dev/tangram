pub use self::{component::Component, data::Data};
use crate::artifact::Artifact;

mod component;
pub mod data;
mod render;
mod unrender;

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Template {
	components: Vec<Component>,
}

impl Template {
	#[must_use]
	pub fn empty() -> Self {
		Self { components: vec![] }
	}

	pub fn new(template: impl Into<Self>) -> Self {
		template.into()
	}

	#[must_use]
	pub fn components(&self) -> &[Component] {
		&self.components
	}

	pub fn artifacts(&self) -> impl Iterator<Item = &Artifact> {
		self.components
			.iter()
			.filter_map(|component| match component {
				Component::Artifact(artifact) => Some(artifact),
				_ => None,
			})
	}
}

impl From<Vec<Component>> for Template {
	fn from(value: Vec<Component>) -> Self {
		Template { components: value }
	}
}

impl From<Component> for Template {
	fn from(value: Component) -> Self {
		vec![value].into()
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

impl FromIterator<Component> for Template {
	fn from_iter<I: IntoIterator<Item = Component>>(value: I) -> Self {
		Template {
			components: value.into_iter().collect(),
		}
	}
}
