use self::{
	clients::blob::Client as BlobClient,
	clients::expression::Client as ExpressionClient,
	heuristics::FILESYSTEM_CONCURRENCY_LIMIT,
	lock::{ExclusiveGuard, Lock, SharedGuard},
};
use crate::{
	db::Db,
	hash::{self, Hash},
	id::Id,
};
use anyhow::Result;
use async_recursion::async_recursion;
pub use options::Options;
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::{Arc, Mutex, Weak},
};
use tokio::sync::Semaphore;

pub mod artifact;
pub mod blob;
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
pub mod watcher;

#[derive(Clone)]
pub struct Builder {
	state: Arc<Lock<State>>,
}

pub struct State {
	/// This is a back reference to the lock that wraps this state.
	lock: Weak<Lock<State>>,

	/// This is the path to the directory where the builder stores its data.
	path: PathBuf,

	/// This is the database used to store expressions and evaluations.
	db: Db,

	/// This HTTP client is for performing HTTP requests when evaluating fetch expressions.
	http_client: reqwest::Client,

	// For expressions that are expensive to evaluate, it is a waste to evaluate them if another evaluation for the same expression is already in progress. This map stores a receiver that will be notified when an in progress evaluation of the expression completes.
	in_progress_evaluations:
		Arc<Mutex<HashMap<Hash, tokio::sync::broadcast::Receiver<Hash>, hash::BuildHasher>>>,

	/// The file system semaphore is used to prevent the builder from opening too many files simultaneously.
	file_system_semaphore: Arc<Semaphore>,

	// The client that connects to the blob server.
	blob_client: Option<BlobClient>,

	// The client that connects to the blob expression server.
	expression_client: Option<ExpressionClient>,
}

impl Builder {
	#[async_recursion]
	#[must_use]
	pub async fn new(path: PathBuf, options: Options) -> Result<Builder> {
		// Ensure the path exists.
		tokio::fs::create_dir_all(&path).await?;

		// Migrate the path.
		Builder::migrate(&path).await?;

		// Create the database.
		let db_path = path.join("db.mdb");
		let db = Db::new(&db_path)?;

		// Create the file system semaphore.
		let file_system_semaphore = Arc::new(Semaphore::new(FILESYSTEM_CONCURRENCY_LIMIT));

		// Create the lock path.
		let lock_path = path.join("lock");

		// Create the HTTP client.
		let http_client = reqwest::Client::new();

		// Create the in progress evaluations.
		let in_progress_evaluations = Arc::new(Mutex::new(HashMap::default()));

		// Create the state.
		let state = Arc::new_cyclic(|builder| {
			let state = State {
				lock: builder.clone(),
				path,
				db,
				http_client,
				in_progress_evaluations,
				file_system_semaphore,
				blob_client: options.blob_client,
				expression_client: options.expression_client,
			};
			Lock::new(lock_path, state)
		});

		// Create the builder.
		let builder = Builder { state };

		Ok(builder)
	}

	pub async fn lock_shared(&self) -> Result<SharedGuard<State>> {
		self.state.lock_shared().await
	}

	pub async fn lock_exclusive(&self) -> Result<ExclusiveGuard<State>> {
		self.state.lock_exclusive().await
	}
}

impl State {
	pub fn builder(&self) -> Builder {
		let state = self.lock.upgrade().unwrap();
		Builder { state }
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
