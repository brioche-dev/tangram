use super::Operation;
use crate::{error::Result, instance::Instance};
use futures::future::try_join_all;

impl Operation {
	pub async fn children(&self, tg: &Instance) -> Result<Vec<Operation>> {
		let children = tg.database.get_operation_children(self.hash())?;
		let children =
			try_join_all(children.into_iter().map(|hash| Operation::get(tg, hash))).await?;
		Ok(children)
	}

	pub fn add_child(&self, tg: &Instance, operation: &Operation) -> Result<()> {
		tg.database
			.add_operation_child(self.hash(), operation.hash())?;
		Ok(())
	}
}
