#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]

pub use self::commands::Args;
use self::dirs::home_directory_path;
use crate::{
	blob::BlobHash,
	database::Database,
	heuristics::SOCKET_SEMAPHORE_SIZE,
	id::Id,
	lock::{ExclusiveGuard, Lock, SharedGuard},
};
use anyhow::{Context, Result};
use api_client::ApiClient;
use compiler::{RequestSender, TrackedFile};
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::{Arc, Mutex},
};
use tokio::sync::{RwLock, Semaphore};
use tokio_util::task::LocalPoolHandle;

pub mod api_client;
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
pub mod package_specifier;
pub mod pull;
pub mod push;
pub mod serve;
pub mod system;
pub mod util;
pub mod value;
pub mod watcher;

#[derive(Clone)]
pub struct Cli {
	inner: Arc<Inner>,
}

struct Inner {
	/// This is the path to the directory where the cli stores its data.
	pub path: PathBuf,

	/// The lock is used to acquire shared and exclusive access to the path.
	pub lock: Lock<()>,

	/// The database is used to store artifacts, packages, and operations.
	pub database: Database,

	/// The file semaphore is used to prevent the cli from opening too many sockets simultaneously.
	pub file_semaphore: Arc<Semaphore>,

	/// The socket semaphore is used to prevent the cli from opening too many files simultaneously.
	pub socket_semaphore: Arc<Semaphore>,

	/// The HTTP client is for performing HTTP requests when running download operations.
	pub http_client: reqwest::Client,

	/// This is a handle to the tokio runtime the CLI was created on.
	pub runtime_handle: tokio::runtime::Handle,

	/// The local pool is for running targets because they are `!Send` futures.
	pub local_pool_handle: LocalPoolHandle,

	/// The compiler request sender is used to send requests to the compiler.
	pub compiler_request_sender: Mutex<Option<RequestSender>>,

	/// The tracked files map tracks files on the filesystem used by the compiler.
	pub tracked_files: RwLock<HashMap<PathBuf, TrackedFile, fnv::FnvBuildHasher>>,

	/// This is the client for communicating with the API.
	pub api_client: ApiClient,
}

static V8_INIT: std::sync::Once = std::sync::Once::new();

impl Cli {
	pub async fn new(path: Option<PathBuf>) -> Result<Cli> {
		// Get the path.
		let path = if let Some(path) = path {
			path
		} else {
			home_directory_path()
				.context("Failed to find the user home directory.")?
				.join(".tangram")
		};

		// Create the lock.
		let lock_path = path.join("lock");
		let lock = Lock::new(lock_path, ());

		// Read the config.
		let config = Self::read_config_from_path(&path.join("config.json")).await?;

		// Read the credentials.
		let credentials = Self::read_credentials_from_path(&path.join("credentials.json")).await?;

		// Resolve the API URL.
		let api_url = config
			.as_ref()
			.and_then(|config| config.api_url.as_ref())
			.cloned();
		let api_url = api_url.unwrap_or_else(|| "https://api.tangram.dev".parse().unwrap());

		// Get the token.
		let token = credentials.map(|credentials| credentials.token);

		// Ensure the path exists.
		tokio::fs::create_dir_all(&path).await?;

		// Migrate the path.
		Self::migrate(&path).await?;

		// Create the database.
		let database_path = path.join("database.mdb");
		let database = Database::new(&database_path)?;

		// Create the file semaphore.
		let file_semaphore = Arc::new(Semaphore::new(SOCKET_SEMAPHORE_SIZE));

		// Create the socket semaphore.
		let socket_semaphore = Arc::new(Semaphore::new(SOCKET_SEMAPHORE_SIZE));

		// Create the HTTP client.
		let http_client = reqwest::Client::new();

		// Create the local pool handle.
		let threads = std::thread::available_parallelism().unwrap().get();
		let local_pool_handle = LocalPoolHandle::new(threads);

		// Initialize v8.
		V8_INIT.call_once(|| {
			// Set the ICU data.
			#[repr(C, align(16))]
			struct IcuData([u8; 10_454_784]);
			static ICU_DATA: IcuData = IcuData(*include_bytes!("../icudtl.dat"));
			v8::icu::set_common_data_71(&ICU_DATA.0).unwrap();

			// Initialize the platform.
			let platform = v8::new_default_platform(0, true);
			v8::V8::initialize_platform(platform.make_shared());

			// Initialize v8.
			v8::V8::initialize();
		});

		// Get a handle to the tokio runtime.
		let runtime_handle = tokio::runtime::Handle::current();

		// Create the API Client.
		let api_client = ApiClient::new(api_url, token);

		// Create the cli.
		let cli = Cli {
			inner: Arc::new(Inner {
				path,
				lock,
				database,
				file_semaphore,
				socket_semaphore,
				http_client,
				local_pool_handle,
				runtime_handle,
				compiler_request_sender: Mutex::new(None),
				tracked_files: RwLock::new(HashMap::default()),
				api_client,
			}),
		};

		Ok(cli)
	}
}

impl Cli {
	pub async fn try_lock_shared(&self) -> Result<Option<SharedGuard<()>>> {
		self.inner.lock.try_lock_shared().await
	}

	pub async fn try_lock_exclusive(&self) -> Result<Option<ExclusiveGuard<()>>> {
		self.inner.lock.try_lock_exclusive().await
	}

	pub async fn lock_shared(&self) -> Result<SharedGuard<()>> {
		self.inner.lock.lock_shared().await
	}

	pub async fn lock_exclusive(&self) -> Result<ExclusiveGuard<()>> {
		self.inner.lock.lock_exclusive().await
	}
}

impl Drop for Inner {
	fn drop(&mut self) {
		// Attempt to shut down the request handler.
		if let Some(sender) = self.compiler_request_sender.lock().unwrap().take() {
			sender.send(None).ok();
		}
	}
}

impl Cli {
	#[must_use]
	pub fn path(&self) -> &Path {
		&self.inner.path
	}

	#[must_use]
	pub fn blobs_path(&self) -> PathBuf {
		self.path().join("blobs")
	}

	#[must_use]
	pub fn checkouts_path(&self) -> PathBuf {
		self.path().join("checkouts")
	}

	#[must_use]
	pub fn config_path(&self) -> PathBuf {
		self.path().join("config.json")
	}

	#[must_use]
	pub fn credentials_path(&self) -> PathBuf {
		self.path().join("credentials.json")
	}

	#[must_use]
	pub fn lock_path(&self) -> PathBuf {
		self.path().join("lock")
	}

	#[must_use]
	pub fn logs_path(&self) -> PathBuf {
		self.path().join("logs")
	}

	#[must_use]
	pub fn temps_path(&self) -> PathBuf {
		self.path().join("temps")
	}

	#[must_use]
	pub fn blob_path(&self, blob_hash: BlobHash) -> PathBuf {
		self.blobs_path().join(blob_hash.to_string())
	}

	#[must_use]
	pub fn temp_path(&self) -> PathBuf {
		self.temps_path().join(Id::generate().to_string())
	}
}
