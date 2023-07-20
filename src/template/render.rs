use super::{Component, Template};
use crate::error::Result;
use futures::{stream::FuturesOrdered, TryStreamExt};
use std::{borrow::Cow, future::Future};

impl Template {
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
		F: FnMut(&'a Component) -> Fut,
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
}
