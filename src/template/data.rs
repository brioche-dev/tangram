use super::Template;
use crate::{
	artifact::{self, Artifact},
	error::{Error, Result},
	instance::Instance,
	placeholder::{self, Placeholder},
};
use futures::future::try_join_all;
use itertools::Itertools;

#[derive(
	Clone,
	Debug,
	Default,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Data {
	#[tangram_serialize(id = 0)]
	pub components: Vec<Component>,
}

#[derive(
	Clone,
	Debug,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Component {
	#[tangram_serialize(id = 0)]
	String(String),

	#[tangram_serialize(id = 1)]
	Artifact(artifact::Hash),

	#[tangram_serialize(id = 2)]
	Placeholder(placeholder::Data),
}

impl Template {
	#[must_use]
	pub fn to_data(&self) -> Data {
		let components = self
			.components
			.iter()
			.map(|component| match component {
				super::Component::String(string) => Component::String(string.clone()),
				super::Component::Artifact(artifact) => Component::Artifact(artifact.hash()),
				super::Component::Placeholder(placeholder) => {
					let placeholder = placeholder.to_data();
					Component::Placeholder(placeholder)
				},
			})
			.collect_vec();
		Data { components }
	}

	pub async fn from_data(tg: &Instance, template: Data) -> Result<Self> {
		let components =
			try_join_all(template.components.into_iter().map(|component| async move {
				let component = match component {
					Component::String(string) => super::Component::String(string),
					Component::Artifact(artifact_hash) => {
						let artifact = Artifact::get(tg, artifact_hash).await?;
						super::Component::Artifact(artifact)
					},
					Component::Placeholder(placeholder) => {
						let placeholder = Placeholder::from_data(placeholder);
						super::Component::Placeholder(placeholder)
					},
				};
				Ok::<_, Error>(component)
			}))
			.await?;
		Ok(Self { components })
	}
}
