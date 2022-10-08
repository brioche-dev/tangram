use super::Shared;
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Create the database pool.
pub(super) fn create_database_pool(path: impl Into<PathBuf>) -> Result<deadpool_sqlite::Pool> {
	deadpool_sqlite::Config {
		path: path.into(),
		pool: Some(deadpool_sqlite::PoolConfig {
			max_size: std::thread::available_parallelism()?.into(),
			timeouts: deadpool_sqlite::Timeouts::new(),
		}),
	}
	.builder(deadpool_sqlite::Runtime::Tokio1)
	.context("Failed to configure the database pool.")?
	.post_create(deadpool_sqlite::Hook::sync_fn(move |conn, _metrics| {
		let error_handler = |error: rusqlite::Error| {
			deadpool_sqlite::HookError::Abort(deadpool_sqlite::HookErrorCause::Backend(error))
		};

		// Lock the connection so we can use it synchronously.
		let conn = conn.lock().map_err(|_| {
			deadpool_sqlite::HookError::Abort(deadpool_sqlite::HookErrorCause::Message(
				"Failed to acquire the database connection lock.".to_owned(),
			))
		})?;

		// Set the analysis limit to 1024 so that `PRAGMA optimize` completes quickly. See <https://www.sqlite.org/pragma.html#pragma_analysis_limit>.
		conn.pragma_update(None, "analysis_limit", 1024)
			.map_err(error_handler)?;

		// Enable incremental auto-vacuum always. See <https://www.sqlite.org/pragma.html#pragma_auto_vacuum>.
		conn.pragma_update(None, "auto_vacuum", "INCREMENTAL")
			.map_err(error_handler)?;

		// Enable WAL journaling. See <https://www.sqlite.org/pragma.html#pragma_journal_mode>.
		conn.pragma_update(None, "journal_mode", "wal")
			.map_err(error_handler)?;

		// Configure disk synchronization for durability. See <https://www.sqlite.org/pragma.html#pragma_synchronous>.
		conn.pragma_update(None, "synchronous", "full")
			.map_err(error_handler)?;

		// Configure F_FULLFSYNC on macOS. See <https://www.sqlite.org/pragma.html#pragma_fullfsync> and <https://www.sqlite.org/pragma.html#pragma_checkpoint_fullfsync>.
		conn.pragma_update(None, "fullfsync", true)
			.map_err(error_handler)?;
		conn.pragma_update(None, "checkpoint_fullfsync", true)
			.map_err(error_handler)?;

		// Configure the page cache size.
		conn.pragma_update(None, "cache_size", -524_288)
			.map_err(error_handler)?;

		Ok(())
	}))
	.post_recycle(deadpool_sqlite::Hook::sync_fn(|_conn, _metrics| {
		// let error_handler = |error: rusqlite::Error| {
		// 	deadpool_sqlite::HookError::Abort(deadpool_sqlite::HookErrorCause::Backend(error))
		// };

		// // Lock the connection so we can use it synchronously.
		// let conn = conn.lock().map_err(|_| {
		// 	deadpool_sqlite::HookError::Abort(deadpool_sqlite::HookErrorCause::Message(
		// 		"Failed to acquire the database connection lock.".to_owned(),
		// 	))
		// })?;

		// // Keep the database optimized. We set `analysis_limit` in the `post_create` hook, so this should be fast. See <https://www.sqlite.org/pragma.html#pragma_optimize>.
		// conn.execute("PRAGMA optimize;", [])
		// 	.map_err(error_handler)?;

		Ok(())
	}))
	.build()
	.context("Failed to create the database pool.")
}

impl Shared {
	/// Call a closure with a database transaction.
	pub(super) async fn database_transaction<'a, T>(
		&self,
		f: impl FnOnce(&rusqlite::Transaction) -> Result<T>,
	) -> Result<T> {
		let database_connection_object = self
			.database_connection_pool
			.get()
			.await
			.context("Failed to retrieve a database connection.")?;
		let mut database_connection = database_connection_object.lock().unwrap();
		let txn = database_connection
			.transaction()
			.context("Failed to start the transaction.")?;
		let output = f(&txn)?;
		txn.commit().context("Failed to commit the transaction.")?;
		Ok(output)
	}
}
