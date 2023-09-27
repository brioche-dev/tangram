use crate::{object, return_error, Result, Server, WrapErr};
use async_recursion::async_recursion;
use futures::{stream::FuturesUnordered, TryStreamExt};

impl Server {
	/// Push an object.
	#[async_recursion]
	#[must_use]
	pub async fn push(&self, id: object::Id) -> Result<()> {
		let Some(parent) = self.state.parent.as_ref() else {
			return_error!("The server does not have a parent.");
		};

		// Attempt to put the object.
		let bytes = self.get_object_bytes(id).await?;
		let result = parent.try_put_object_bytes(id, &bytes).await?;

		match result {
			// If the object was added, then return.
			Ok(_) => return Ok(()),

			// Otherwise, push the missing children.
			Err(children) => {
				children
					.into_iter()
					.map(|id| self.push(id))
					.collect::<FuturesUnordered<_>>()
					.try_collect()
					.await?;
			},
		}

		// Attempt to put the object again. This time, return an error if there are missing children.
		parent
			.try_put_object_bytes(id, &bytes)
			.await?
			.ok()
			.wrap_err("Failed to push the block.")?;

		Ok(())
	}
}
