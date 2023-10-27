use super::Server;
use lmdb::Transaction;
use tangram_client as tg;
use tangram_util::http::{empty, Incoming, Outgoing};
use tg::{Result, WrapErr};

impl Server {
	pub async fn handle_clean_request(
		&self,
		_request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		self.clean().await?;
		Ok(http::Response::builder()
			.status(http::StatusCode::OK)
			.body(empty())
			.unwrap())
	}

	pub async fn clean(&self) -> Result<()> {
		// Clear the database.
		{
			let mut txn = self
				.state
				.database
				.env
				.begin_rw_txn()
				.wrap_err("Failed to begin a transaction.")?;
			txn.clear_db(self.state.database.objects)
				.wrap_err("Failed to clear the objects.")?;
			txn.clear_db(self.state.database.assignments)
				.wrap_err("Failed to clear the assignments.")?;
			txn.commit().wrap_err("Failed to commit the transaction.")?;
		}

		// Clear the temporary path.
		tokio::fs::remove_dir_all(self.temps_path())
			.await
			.wrap_err("Failed to remove the temporary directory.")?;
		tokio::fs::create_dir_all(self.temps_path())
			.await
			.wrap_err("Failed to recreate the temporary directory.")?;

		Ok(())
	}
}
