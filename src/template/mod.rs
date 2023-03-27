pub use self::render::{Mode, Output, Path};
use crate::{artifact, error::Result, placeholder::Placeholder};
use futures::future::try_join_all;
use std::{borrow::Cow, future::Future};

mod render;
mod unrender;

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Template {
	#[buffalo(id = 0)]
	pub components: Vec<Component>,
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
#[serde(tag = "kind", content = "value")]
pub enum Component {
	#[buffalo(id = 0)]
	#[serde(rename = "string")]
	String(String),

	#[buffalo(id = 1)]
	#[serde(rename = "artifact")]
	Artifact(artifact::Hash),

	#[buffalo(id = 2)]
	#[serde(rename = "placeholder")]
	Placeholder(Placeholder),
}

impl Template {
	pub async fn render<'a, F, Fut>(&'a self, f: F) -> Result<String>
	where
		F: FnMut(&'a Component) -> Fut,
		Fut: Future<Output = Result<Cow<'a, str>>>,
	{
		Ok(try_join_all(self.components.iter().map(f)).await?.join(""))
	}
}
