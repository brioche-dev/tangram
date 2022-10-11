use self::{
	clients::blob::Client as BlobClient, clients::expression::Client as ExpressionClient,
	heuristics::FILESYSTEM_CONCURRENCY_LIMIT, lock::Lock,
};
use crate::{hash::Hash, id::Id};
use anyhow::Result;
use async_recursion::async_recursion;
use fnv::FnvBuildHasher;
use std::{
	collections::HashMap,
	num::NonZeroUsize,
	path::{Path, PathBuf},
	sync::{Arc, Mutex},
};
use tokio::sync::Semaphore;

pub mod artifact;
pub mod blob;
pub mod cache;
pub mod checkin;
pub mod checkout;
pub mod clients;
pub mod evaluate;
pub mod expression;
pub mod gc;
pub mod heuristics;
pub mod lock;
pub mod migrations;
pub mod options;
pub mod package;
pub mod pull;
pub mod push;

pub use options::Options;

#[derive(Clone)]
pub struct Builder {
	state: Arc<Lock<State>>,
}

pub type Shared = lock::SharedGuard<State>;

pub type Exclusive = lock::ExclusiveGuard<State>;

pub struct State {
	/// This is the path to the directory where the builder stores its data.
	path: PathBuf,

	/// This is the LMDB env.
	env: lmdb::Environment,

	/// This is the expressions database.
	expressions_db: lmdb::Database,

	/// This is the evaluations database.
	evaluations_db: lmdb::Database,

	/// This HTTP client is for performing HTTP requests when evaluating fetch expressions.
	http_client: reqwest::Client,

	// For expressions that are expensive to evaluate, it is a waste to evaluate them if another evaluation for the same expression is already in progress. This map stores a receiver that will be notified when an in progress evaluation of the expression completes.
	pub in_progress_evaluations:
		Arc<Mutex<HashMap<Hash, tokio::sync::broadcast::Receiver<Hash>, FnvBuildHasher>>>,

	// This is a pool for spawning !Send tasks.
	local_pool_handle: tokio_util::task::LocalPoolHandle,

	/// The file system semaphore is used to prevent the builder from opening too many files simultaneously.
	file_system_semaphore: Arc<Semaphore>,

	// The client that connects to the blob server.
	blob_client: BlobClient,

	// The client that connects to the blob expression server.
	expression_client: ExpressionClient,
}

impl Builder {
	#[async_recursion]
	#[must_use]
	pub async fn new(path: PathBuf, options: Options) -> Result<Builder> {
		// Ensure the path exists.
		tokio::fs::create_dir_all(&path).await?;

		// Migrate the path.
		Builder::migrate(&path).await?;

		// Create the env.
		let database_path = path.join("db.mdb");
		let mut env_builder = lmdb::Environment::new();
		env_builder.set_max_dbs(2);
		env_builder.set_flags(lmdb::EnvironmentFlags::NO_SUB_DIR);
		let env = env_builder.open(&database_path)?;

		// Open the expression db.
		let expressions_db = env.open_db("expressions".into())?;

		// Open the evaluations db.
		let evaluations_db = env.open_db("evaluations".into())?;

		// Create the file system semaphore.
		let file_system_semaphore = Arc::new(Semaphore::new(FILESYSTEM_CONCURRENCY_LIMIT));

		// Create the lock path.
		let lock_path = path.join("lock");

		// Create the HTTP client.
		let http_client = reqwest::Client::new();

		// Create the local pool.
		let available_parallelism = std::thread::available_parallelism()
			.unwrap_or_else(|_| NonZeroUsize::new(1).unwrap())
			.into();
		let local_pool_handle = tokio_util::task::LocalPoolHandle::new(available_parallelism);

		// Create the in progress evaluations.
		let in_progress_evaluations = Arc::new(Mutex::new(HashMap::default()));

		// Create the state.
		let state = State {
			path,
			env,
			expressions_db,
			evaluations_db,
			http_client,
			in_progress_evaluations,
			local_pool_handle,
			file_system_semaphore,
			blob_client: options.blob_client,
			expression_client: options.expression_client,
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
