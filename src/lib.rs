#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub use self::commands::Args;
use self::dirs::home_directory_path;
use crate::{
	api_client::ApiClient,
	clients::{blob::Client as BlobClient, expression::Client as ExpressionClient},
	db::Db,
	hash::Hash,
	heuristics::FILESYSTEM_CONCURRENCY_LIMIT,
	id::Id,
	lock::{ExclusiveGuard, Lock, SharedGuard},
};
use anyhow::{Context, Result};
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::{Arc, Mutex, Weak},
};
use tokio::sync::Semaphore;
use tokio_util::task::LocalPoolHandle;

pub mod api_client;
pub mod artifact;
pub mod blob;
pub mod checkin;
pub mod checkout;
pub mod checksum;
pub mod clients;
pub mod commands;
pub mod compiler;
pub mod config;
pub mod credentials;
pub mod db;
pub mod dirs;
pub mod evaluate;
pub mod expression;
pub mod gc;
pub mod hash;
pub mod heuristics;
pub mod id;
pub mod lock;
pub mod lockfile;
pub mod lsp;
pub mod manifest;
pub mod migrations;
pub mod operation;
pub mod package;
pub mod pull;
pub mod push;
pub mod server;
pub mod specifier;
pub mod system;
pub mod util;
pub mod value;
pub mod watcher;

#[derive(Clone)]
pub struct Cli {
	state: Arc<Lock<State>>,
}

pub struct State {
	/// This is a weak reference to the lock that wraps this state.
	pub state: Weak<Lock<State>>,

	/// This is the path to the directory where the cli stores its data.
	pub path: PathBuf,

	/// This is the database used to store expressions and evaluations.
	pub db: Db,

	/// For expressions that are expensive to evaluate, it is a waste to evaluate them if another evaluation for the same expression is already in progress. This map stores a receiver that will be notified when an in progress evaluation of the expression completes.
	pub in_progress_evaluations:
		Arc<Mutex<HashMap<Hash, tokio::sync::broadcast::Receiver<Hash>, hash::BuildHasher>>>,

	/// The file system semaphore is used to prevent the cli from opening too many files simultaneously.
	pub file_system_semaphore: Arc<Semaphore>,

	/// The client that connects to the blob server.
	pub blob_client: Option<BlobClient>,

	/// The client that connects to the blob expression server.
	pub expression_client: Option<ExpressionClient>,

	/// This HTTP client is for performing HTTP requests when evaluating download expressions.
	pub http_client: reqwest::Client,

	/// This local pool handle is for spawning `!Send` futures.
	pub local_pool_handle: LocalPoolHandle,

	/// The API client is used to communicate with the API.
	pub api_client: ApiClient,
}

static V8_INIT: std::sync::Once = std::sync::Once::new();

impl Cli {
	pub async fn new() -> Result<Cli> {
		// Get the path.
		let path = Self::path()?;

		// Read the config.
		let config = Self::read_config().await?;

		// Read the credentials.
		let credentials = Self::read_credentials().await?;

		// Resolve the API URL.
		let api_url = config
			.as_ref()
			.and_then(|config| config.api_url.as_ref())
			.cloned();
		let api_url = api_url.unwrap_or_else(|| "https://api.tangram.dev".parse().unwrap());

		// Get the token.
		let token = credentials.map(|credentials| credentials.token);

		// Create the API Client.
		let api_client = ApiClient::new(api_url.clone(), token.clone());

		// Create the blob client.
		let blob_client = clients::blob::Client::new(api_url.clone(), token.clone());

		// Create the expression client.
		let expression_client = clients::expression::Client::new(api_url.clone(), token.clone());

		// Ensure the path exists.
		tokio::fs::create_dir_all(&path).await?;

		// Migrate the path.
		Self::migrate(&path).await?;

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

		// Create the local pool handle.
		let threads = std::thread::available_parallelism().unwrap().get();
		let local_pool_handle = LocalPoolHandle::new(threads);

		// Initialize v8.
		V8_INIT.call_once(|| {
			#[repr(C, align(16))]
			struct IcuData([u8; 10_454_784]);
			static ICU_DATA: IcuData = IcuData(*include_bytes!("../icudtl.dat"));
			v8::icu::set_common_data_71(&ICU_DATA.0).unwrap();
			let platform = v8::new_default_platform(0, true);
			v8::V8::initialize_platform(platform.make_shared());
			v8::V8::initialize();
		});

		// Create the state.
		let state = Arc::new_cyclic(|state| {
			let state = State {
				state: state.clone(),
				path,
				db,
				http_client,
				in_progress_evaluations,
				file_system_semaphore,
				blob_client: Some(blob_client),
				expression_client: Some(expression_client),
				local_pool_handle,
				api_client,
			};
			Lock::new(lock_path, state)
		});

		// Create the cli.
		let cli = Cli { state };

		Ok(cli)
	}

	fn path() -> Result<PathBuf> {
		Ok(home_directory_path()
			.context("Failed to find the user home directory.")?
			.join(".tangram"))
	}
}

impl Cli {
	pub async fn lock_shared(&self) -> Result<SharedGuard<State>> {
		self.state.lock_shared().await
	}

	pub async fn lock_exclusive(&self) -> Result<ExclusiveGuard<State>> {
		self.state.lock_exclusive().await
	}
}

impl State {
	pub fn upgrade(&self) -> Cli {
		let state = self.state.upgrade().unwrap();
		Cli { state }
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
	pub fn blobs_path(&self) -> PathBuf {
		self.path.join("blobs")
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
	pub fn blob_path(&self, blob_hash: Hash) -> PathBuf {
		self.path.join("blobs").join(blob_hash.to_string())
	}

	#[must_use]
	pub fn create_temp_path(&self) -> PathBuf {
		self.path.join("temps").join(Id::generate().to_string())
	}
}
