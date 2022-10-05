use crate::{
	client::Client, db::create_database_pool, evaluators::Evaluator,
	heuristics::FILESYSTEM_CONCURRENCY_LIMIT, id::Id, lock, lock::Lock, options::Options,
};
use anyhow::Result;
use async_recursion::async_recursion;
use std::{
	path::{Path, PathBuf},
	sync::Arc,
};
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct Builder {
	state: Arc<Lock<State>>,
}

pub type Shared = lock::SharedGuard<State>;

pub type Exclusive = lock::ExclusiveGuard<State>;

pub struct State {
	/// This is the path to the directory where the builder stores its data.
	pub path: PathBuf,

	/// This is the connection pool for the builder's SQLite database.
	pub database_connection_pool: deadpool_sqlite::Pool,

	/// The builder will use these peers to get blobs and expressions and to evaluate expressions.
	pub peers: Vec<Client>,

	/// These are the evaluators.
	pub evaluators: Vec<Box<dyn Send + Sync + Evaluator>>,

	/// The file system semaphore is used to prevent the builder from opening too many files simultaneously.
	pub file_system_semaphore: Arc<Semaphore>,
}

impl Builder {
	#[async_recursion]
	#[must_use]
	pub async fn new(options: Options) -> Result<Builder> {
		// Ensure the path exists.
		let path = options.path;
		tokio::fs::create_dir_all(&path).await?;

		// Migrate the path.
		Builder::migrate(&path).await?;

		// Create the database pool.
		let database_path = path.join("db.sqlite3");
		let database_connection_pool =
			tokio::task::block_in_place(|| create_database_pool(database_path))?;

		// Create the peer clients.
		let peers = options
			.peers
			.into_iter()
			.map(|url| Client::new(url, None))
			.collect();

		// Create the evaluators.
		let evaluators: Vec<Box<dyn Send + Sync + Evaluator>> = vec![
			Box::new(crate::evaluators::array::Array::new()),
			Box::new(crate::evaluators::fetch::Fetch::new()),
			Box::new(crate::evaluators::package::Package::new()),
			Box::new(crate::evaluators::map::Map::new()),
			Box::new(crate::evaluators::primitive::Primitive::new()),
			Box::new(crate::evaluators::js::Js::new()),
			Box::new(crate::evaluators::process::Process::new()),
			Box::new(crate::evaluators::target::Target::new()),
			Box::new(crate::evaluators::template::Template::new()),
		];

		// Create the file system semaphore.
		let file_system_semaphore = Arc::new(Semaphore::new(FILESYSTEM_CONCURRENCY_LIMIT));

		// Create the lock path.
		let lock_path = path.join("lock");

		// Create the state.
		let state = State {
			path,
			database_connection_pool,
			peers,
			evaluators,
			file_system_semaphore,
		};

		// Create the server.
		let state = Arc::new(Lock::new(lock_path, state));
		let server = Builder { state };

		Ok(server)
	}
}

impl Builder {
	pub async fn lock_shared(&self) -> Result<Shared> {
		self.state.lock_shared().await
	}

	pub async fn lock_exclusive(&self) -> Result<Exclusive> {
		self.state.lock_exclusive().await
	}
}

impl State {
	#[must_use]
	pub fn path(&self) -> &Path {
		&self.path
	}

	#[must_use]
	pub fn artifacts_path(&self) -> PathBuf {
		self.path.join("artifacts")
	}

	#[must_use]
	pub fn lock_path(&self) -> PathBuf {
		self.path.join("lock")
	}

	#[must_use]
	pub fn temps_path(&self) -> PathBuf {
		self.path.join("temps")
	}

	#[must_use]
	pub fn create_temp_path(&self) -> PathBuf {
		self.path.join("temps").join(Id::generate().to_string())
	}
}
