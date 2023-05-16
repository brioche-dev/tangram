use super::{Component, Template};
use crate::error::Result;
use futures::future::try_join_all;
use std::{borrow::Cow, future::Future};

impl Template {
	pub fn render_sync<'a, F>(&'a self, mut f: F) -> Result<String>
	where
		F: (FnMut(&'a Component) -> Result<Cow<'a, str>>) + 'a,
	{
		let mut string = String::new();
		for component in &self.components {
			string.push_str(&f(component)?);
		}
		Ok(string)
	}

	pub async fn render<'a, F, Fut>(&'a self, f: F) -> Result<String>
	where
		F: FnMut(&'a Component) -> Fut,
		Fut: Future<Output = Result<Cow<'a, str>>>,
	{
		Ok(try_join_all(self.components.iter().map(f)).await?.join(""))
	}
}
