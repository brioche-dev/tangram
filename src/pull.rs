use crate::{object, return_error, Result, Server};
use async_recursion::async_recursion;
use futures::{stream::FuturesUnordered, TryStreamExt};

impl Server {
	/// Pull an object.
	#[async_recursion]
	#[must_use]
	pub async fn pull(&self, id: object::Id) -> Result<()> {
		// Get the object.
		let Some(bytes) = self.try_get_object_bytes(id).await? else {
			return_error!(r#"Failed to get the value "{id}"."#);
		};

		// Get the values's children.
		let data = object::Data::deserialize(id, &bytes)?;
		data.children()
			.into_iter()
			.map(|id| self.pull(id))
			.collect::<FuturesUnordered<_>>()
			.try_collect()
			.await?;

		Ok(())
	}
}
