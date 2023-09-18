use crate::{error, return_error, Id, Result, Server};
use async_recursion::async_recursion;
use futures::{stream::FuturesUnordered, TryStreamExt};

impl Server {
	/// Push a value.
	#[async_recursion]
	#[must_use]
	pub async fn push(&self, id: Id) -> Result<()> {
		let Some(parent) = self.state.parent.as_ref() else {
			return_error!("The server does not have a parent.");
		};

		// Attempt to put the value.
		let bytes = self.get_value_bytes(id).await?;
		let result = parent.try_put_value_bytes(id, &bytes).await?;

		match result {
			// If the value was added, then return.
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

		// Attempt to put the value again. This time, return an error if there are missing children.
		parent
			.try_put_value_bytes(id, &bytes)
			.await?
			.map_err(|_| error!("Failed to push the block."))?;

		Ok(())
	}
}
