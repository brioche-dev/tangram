use super::Server;
use anyhow::{Context, Result};
use std::{path::PathBuf, sync::Arc};

impl Server {
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
		.post_recycle(deadpool_sqlite::Hook::sync_fn(|conn, _metrics| {
			let error_handler = |error: rusqlite::Error| {
				deadpool_sqlite::HookError::Abort(deadpool_sqlite::HookErrorCause::Backend(error))
			};

			// Lock the connection so we can use it synchronously.
			let conn = conn.lock().map_err(|_| {
				deadpool_sqlite::HookError::Abort(deadpool_sqlite::HookErrorCause::Message(
					"Failed to acquire the database connection lock.".to_owned(),
				))
			})?;

			// Keep the database optimized. We set `analysis_limit` in the `post_create` hook, so this should be fast. See <https://www.sqlite.org/pragma.html#pragma_optimize>.
			conn.execute("PRAGMA optimize;", [])
				.map_err(error_handler)?;

			Ok(())
		}))
		.build()
		.context("Failed to create the database pool.")
	}

	pub(super) async fn database_execute(
		self: &Arc<Self>,
		sql: &str,
		params: impl rusqlite::Params,
	) -> Result<()> {
		let database_connection_object = self
			.database_connection_pool
			.get()
			.await
			.context("Failed to retrieve a database connection.")?;
		tokio::task::block_in_place(move || -> Result<_> {
			let database_connection = database_connection_object.lock().unwrap();
			database_connection.execute(sql, params)?;
			Ok(())
		})
	}

	pub(super) async fn database_query_row<T, P, F>(
		self: &Arc<Self>,
		sql: &str,
		params: P,
		f: F,
	) -> Result<Option<T>>
	where
		P: rusqlite::Params,
		F: FnOnce(&rusqlite::Row<'_>) -> Result<T>,
	{
		let database_connection_object = self
			.database_connection_pool
			.get()
			.await
			.context("Failed to retrieve a database connection.")?;
		tokio::task::block_in_place(move || -> Result<_> {
			let database_connection = database_connection_object.lock().unwrap();
			let mut statement = database_connection
				.prepare_cached(sql)
				.context("Failed to prepare the query.")?;
			let mut rows = statement
				.query(params)
				.context("Failed to execute the query.")?;
			let maybe_row = rows.next().context("Failed to fetch a row.")?;
			let result: Option<T> = maybe_row.map(f).transpose()?;
			Ok(result)
		})
	}

	pub(super) async fn database_query_rows<T, P, F>(
		self: &Arc<Self>,
		sql: &str,
		params: P,
		f: F,
	) -> Result<Vec<T>>
	where
		P: rusqlite::Params,
		F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
	{
		let database_connection_object = self
			.database_connection_pool
			.get()
			.await
			.context("Failed to retrieve a database connection.")?;
		tokio::task::block_in_place(move || -> Result<_> {
			let database_connection = database_connection_object.lock().unwrap();
			let mut statement = database_connection
				.prepare_cached(sql)
				.context("Failed to prepare the query.")?;
			let rows = statement
				.query_map(params, f)
				.context("Failed to execute the query.")?
				.collect::<rusqlite::Result<Vec<_>>>()?;
			Ok(rows)
		})
	}
}
