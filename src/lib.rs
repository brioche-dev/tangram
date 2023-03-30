#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

use self::{
	constants::{FILE_SEMAPHORE_SIZE, SOCKET_SEMAPHORE_SIZE},
	database::Database,
	error::Result,
	lock::Lock,
	util::{fs, task_map::TaskMap},
	value::Value,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Semaphore;
use tokio_util::task::LocalPoolHandle;
use url::Url;

pub mod api;
pub mod archive;
pub mod artifact;
pub mod blob;
pub mod call;
pub mod checkin;
pub mod checkout;
pub mod checksum;
pub mod clean;
pub mod client;
pub mod constants;
pub mod database;
pub mod directory;
pub mod download;
pub mod error;
pub mod file;
pub mod function;
pub mod hash;
pub mod id;
pub mod language;
pub mod lock;
pub mod lockfile;
pub mod log;
pub mod lsp;
pub mod metadata;
pub mod migrations;
pub mod module;
pub mod operation;
pub mod package;
pub mod path;
pub mod placeholder;
pub mod process;
pub mod pull;
pub mod push;
pub mod server;
pub mod symlink;
pub mod system;
pub mod temp;
pub mod template;
pub mod util;
pub mod value;

pub struct Instance {
	/// The directory where all data is stored.
	path: fs::PathBuf,

	/// A lock used to acquire shared and exclusive access to the path.
	lock: Lock<()>,

	/// The database used to store artifacts, packages, and operations.
	database: Database,

	/// A semaphore used to prevent opening too many files simultaneously.
	file_semaphore: Arc<Semaphore>,

	/// A semaphore used to prevent opening too many sockets simultaneously.
	socket_semaphore: Arc<Semaphore>,

	/// An HTTP client for download operations.
	http_client: reqwest::Client,

	/// A handle to the main tokio runtime.
	runtime_handle: tokio::runtime::Handle,

	/// A local pool for running `!Send` futures.
	local_pool_handle: LocalPoolHandle,

	/// A channel sender to send requests to the language service.
	language_service_request_sender: std::sync::Mutex<Option<language::service::RequestSender>>,

	/// A map that tracks packages on disk.
	package_trackers:
		tokio::sync::RwLock<HashMap<artifact::Hash, fs::PathBuf, fnv::FnvBuildHasher>>,

	/// A task map that deduplicates internal checkouts.
	#[allow(clippy::type_complexity)]
	internal_checkouts_task_map:
		std::sync::Mutex<Option<Arc<TaskMap<artifact::Hash, Result<fs::PathBuf>>>>>,

	/// A task map that deduplicates operations.
	#[allow(clippy::type_complexity)]
	operations_task_map: std::sync::Mutex<Option<Arc<TaskMap<operation::Hash, Result<Value>>>>>,

	/// A map that tracks changes to modules in memory.
	documents:
		tokio::sync::RwLock<HashMap<module::Identifier, module::Document, fnv::FnvBuildHasher>>,

	/// A map that tracks changes to modules on disk.
	module_trackers:
		tokio::sync::RwLock<HashMap<module::Identifier, module::Tracker, fnv::FnvBuildHasher>>,

	/// A client for communicating with the API.
	api_client: api::Client,
}

#[derive(Clone, Debug, Default)]
pub struct Options {
	pub api_url: Option<Url>,
	pub api_token: Option<String>,
}

static V8_INIT: std::sync::Once = std::sync::Once::new();

fn initialize_v8() {
	// Set the ICU data.
	#[repr(C, align(16))]
	struct IcuData([u8; 10_541_264]);
	static ICU_DATA: IcuData = IcuData(*include_bytes!(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/assets/icudtl.dat"
	)));
	v8::icu::set_common_data_72(&ICU_DATA.0).unwrap();

	// Initialize the platform.
	let platform = v8::new_default_platform(0, true);
	v8::V8::initialize_platform(platform.make_shared());

	// Initialize V8.
	v8::V8::initialize();
}

impl Instance {
	pub async fn new(path: fs::PathBuf, options: Options) -> Result<Instance> {
		// Initialize V8.
		V8_INIT.call_once(initialize_v8);

		// Create the lock.
		let lock_path = path.join("lock");
		let lock = Lock::new(&lock_path, ());

		// Ensure the path exists.
		tokio::fs::create_dir_all(&path).await?;

		// Migrate the path.
		Self::migrate(&path).await?;

		// Open the database.
		let database_path = path.join("database.mdb");
		let database = Database::open(&database_path)?;

		// Create the file semaphore.
		let file_semaphore = Arc::new(Semaphore::new(FILE_SEMAPHORE_SIZE));

		// Create the socket semaphore.
		let socket_semaphore = Arc::new(Semaphore::new(SOCKET_SEMAPHORE_SIZE));

		// Create the HTTP client.
		let http_client = reqwest::Client::new();

		// Create the local pool handle.
		let threads = std::thread::available_parallelism().unwrap().get();
		let local_pool_handle = LocalPoolHandle::new(threads);

		// Get a handle to the tokio runtime.
		let runtime_handle = tokio::runtime::Handle::current();

		// Create the language service request sender.
		let language_service_request_sender = std::sync::Mutex::new(None);

		// Create the package trackers map.
		let package_trackers = tokio::sync::RwLock::new(HashMap::default());

		// Create the internal checkouts task map.
		let internal_checkouts_task_map = std::sync::Mutex::new(None);

		// Create the operations task map.
		let operations_task_map = std::sync::Mutex::new(None);

		// Create the documents map.
		let documents = tokio::sync::RwLock::new(HashMap::default());

		// Create the module trackers map.
		let module_trackers = tokio::sync::RwLock::new(HashMap::default());

		// Create the API Client.
		let api_url = options
			.api_url
			.unwrap_or_else(|| "https://api.tangram.dev".parse().unwrap());
		let token = options.api_token;
		let api_client = api::Client::new(api_url, token);

		// Create the instance.
		let instance = Instance {
			path,
			lock,
			database,
			file_semaphore,
			socket_semaphore,
			http_client,
			runtime_handle,
			local_pool_handle,
			language_service_request_sender,
			package_trackers,
			internal_checkouts_task_map,
			operations_task_map,
			documents,
			module_trackers,
			api_client,
		};

		Ok(instance)
	}
}

impl Instance {
	pub async fn try_lock_shared(&self) -> Result<Option<lock::SharedGuard<()>>> {
		self.lock.try_lock_shared().await
	}

	pub async fn try_lock_exclusive(&self) -> Result<Option<lock::ExclusiveGuard<()>>> {
		self.lock.try_lock_exclusive().await
	}

	pub async fn lock_shared(&self) -> Result<lock::SharedGuard<()>> {
		self.lock.lock_shared().await
	}

	pub async fn lock_exclusive(&self) -> Result<lock::ExclusiveGuard<()>> {
		self.lock.lock_exclusive().await
	}
}

impl Instance {
	#[must_use]
	pub fn path(&self) -> &fs::Path {
		&self.path
	}

	#[must_use]
	pub fn artifacts_path(&self) -> fs::PathBuf {
		self.path().join("artifacts")
	}

	#[must_use]
	pub fn artifact_path(&self, artifact_hash: artifact::Hash) -> fs::PathBuf {
		self.artifacts_path().join(artifact_hash.to_string())
	}

	#[must_use]
	pub fn blobs_path(&self) -> fs::PathBuf {
		self.path().join("blobs")
	}

	#[must_use]
	pub fn blob_path(&self, blob_hash: blob::Hash) -> fs::PathBuf {
		self.blobs_path().join(blob_hash.to_string())
	}

	#[must_use]
	pub fn database_path(&self) -> fs::PathBuf {
		self.path().join("database.mdb")
	}

	#[must_use]
	pub fn logs_path(&self) -> fs::PathBuf {
		self.path().join("logs")
	}

	#[must_use]
	pub fn log_path(&self, operation_hash: operation::Hash) -> fs::PathBuf {
		self.logs_path().join(operation_hash.to_string())
	}

	#[must_use]
	pub fn temps_path(&self) -> fs::PathBuf {
		self.path().join("temps")
	}
}

impl Instance {
	pub fn api_client(&self) -> &api::Client {
		&self.api_client
	}
}
