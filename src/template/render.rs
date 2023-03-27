use super::{Component, Template};
use crate::error::Result;
use futures::future::try_join_all;
use std::{borrow::Cow, future::Future};

impl Template {
	pub async fn render<'a, F, Fut>(&'a self, f: F) -> Result<String>
	where
		F: FnMut(&'a Component) -> Fut,
		Fut: Future<Output = Result<Cow<'a, str>>>,
	{
		Ok(try_join_all(self.components.iter().map(f)).await?.join(""))
	}
}
