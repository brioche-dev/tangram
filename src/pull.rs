use crate::{return_error, server::Server, value, Id, Result};
use async_recursion::async_recursion;
use futures::{stream::FuturesUnordered, TryStreamExt};

impl Server {
	/// Pull a value.
	#[async_recursion]
	#[must_use]
	pub async fn pull(&self, id: Id) -> Result<()> {
		// Get the value.
		let Some(bytes) = self.try_get_value_bytes(id).await? else {
			return_error!(r#"Failed to get the value "{id}"."#);
		};

		// Get the values's children.
		let data = value::Data::deserialize(&bytes)?;
		data.children()
			.into_iter()
			.map(|id| self.pull(id))
			.collect::<FuturesUnordered<_>>()
			.try_collect()
			.await?;

		Ok(())
	}
}
