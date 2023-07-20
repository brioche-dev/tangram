use super::Template;
use crate::{
	artifact::Artifact,
	block::Block,
	error::{Error, Result},
	instance::Instance,
	placeholder::{self, Placeholder},
};
use futures::{stream::FuturesOrdered, TryStreamExt};

#[derive(
	Clone,
	Debug,
	Default,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
pub struct Data {
	#[tangram_serialize(id = 0)]
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
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Component {
	#[tangram_serialize(id = 0)]
	String(String),

	#[tangram_serialize(id = 1)]
	Artifact(Block),

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
				super::Component::Artifact(artifact) => Component::Artifact(artifact.block()),
				super::Component::Placeholder(placeholder) => {
					let placeholder = placeholder.to_data();
					Component::Placeholder(placeholder)
				},
			})
			.collect();
		Data { components }
	}

	pub async fn from_data(tg: &Instance, template: Data) -> Result<Self> {
		let components = template
			.components
			.into_iter()
			.map(|component| async move {
				let component = match component {
					Component::String(string) => super::Component::String(string),
					Component::Artifact(block) => {
						let artifact = Artifact::get(tg, block).await?;
						super::Component::Artifact(artifact)
					},
					Component::Placeholder(placeholder) => {
						let placeholder = Placeholder::from_data(placeholder);
						super::Component::Placeholder(placeholder)
					},
				};
				Ok::<_, Error>(component)
			})
			.collect::<FuturesOrdered<_>>()
			.try_collect()
			.await?;
		Ok(Self { components })
	}
}
