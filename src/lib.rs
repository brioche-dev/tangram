#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub use self::commands::Args;
use self::dirs::home_directory_path;
use crate::{
	blob::BlobHash,
	database::Database,
	heuristics::FILESYSTEM_CONCURRENCY_LIMIT,
	id::Id,
	lock::{ExclusiveGuard, Lock, SharedGuard},
};
use anyhow::{Context, Result};
use std::{
	path::{Path, PathBuf},
	sync::{Arc, Weak},
};
use tokio::sync::Semaphore;
use tokio_util::task::LocalPoolHandle;

pub mod artifact;
pub mod blob;
pub mod checkin;
pub mod checkout;
pub mod checksum;
pub mod client;
pub mod commands;
pub mod compiler;
pub mod config;
pub mod credentials;
pub mod database;
pub mod dirs;
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

	/// This is the database used to store artifacts, package, operations, evaluations, and outputs.
	pub database: Database,

	/// The file system semaphore is used to prevent the cli from opening too many files simultaneously.
	pub file_system_semaphore: Arc<Semaphore>,

	/// This HTTP client is for performing HTTP requests when running download operations.
	pub http_client: reqwest::Client,

	/// This local pool handle is for spawning `!Send` futures, such as targets.
	pub local_pool_handle: LocalPoolHandle,
}

static V8_INIT: std::sync::Once = std::sync::Once::new();

impl Cli {
	pub async fn new() -> Result<Cli> {
		// Get the path.
		let path = Self::path()?;

		// // Read the config.
		// let config = Self::read_config().await?;

		// // Read the credentials.
		// let credentials = Self::read_credentials().await?;

		// // Resolve the API URL.
		// let api_url = config
		// 	.as_ref()
		// 	.and_then(|config| config.api_url.as_ref())
		// 	.cloned();
		// let api_url = api_url.unwrap_or_else(|| "https://api.tangram.dev".parse().unwrap());

		// // Get the token.
		// let token = credentials.map(|credentials| credentials.token);

		// Ensure the path exists.
		tokio::fs::create_dir_all(&path).await?;

		// Migrate the path.
		Self::migrate(&path).await?;

		// Create the database.
		let database_path = path.join("database.mdb");
		let database = Database::new(&database_path)?;

		// Create the file system semaphore.
		let file_system_semaphore = Arc::new(Semaphore::new(FILESYSTEM_CONCURRENCY_LIMIT));

		// Create the lock path.
		let lock_path = path.join("lock");

		// Create the HTTP client.
		let http_client = reqwest::Client::new();

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
				database,
				http_client,
				file_system_semaphore,
				local_pool_handle,
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
	pub fn blob_path(&self, blob_hash: BlobHash) -> PathBuf {
		self.path.join("blobs").join(blob_hash.to_string())
	}

	#[must_use]
	pub fn create_temp_path(&self) -> PathBuf {
		self.path.join("temps").join(Id::generate().to_string())
	}
}
