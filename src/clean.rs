use super::Server;
use crate::{Id, Result, WrapErr};

impl Server {
	pub async fn clean(&self, _roots: Vec<Id>) -> Result<()> {
		// Delete all temps.
		tokio::fs::remove_dir_all(&self.temps_path())
			.await
			.wrap_err("Failed to delete the temps directory.")?;
		tokio::fs::create_dir_all(&self.temps_path())
			.await
			.wrap_err("Failed to recreate the temps directory.")?;

		// TODO

		Ok(())
	}
}
