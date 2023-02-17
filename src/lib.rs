#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub use self::commands::Args;
use self::{
	constants::{FILE_SEMAPHORE_SIZE, SOCKET_SEMAPHORE_SIZE},
	database::Database,
	id::Id,
	language::service::RequestSender,
	lock::Lock,
	system::System,
	value::Value,
};
use anyhow::Result;
use std::{
	collections::{BTreeMap, HashMap},
	sync::Arc,
};
use tokio::sync::Semaphore;
use tokio_util::task::LocalPoolHandle;

pub mod api;
pub mod artifact;
pub mod blob;
pub mod call;
pub mod checkin;
pub mod checkout;
pub mod checksum;
pub mod clean;
pub mod client;
pub mod commands;
pub mod config;
pub mod constants;
pub mod credentials;
pub mod database;
pub mod directory;
pub mod download;
pub mod file;
pub mod function;
pub mod hash;
pub mod id;
pub mod language;
pub mod lock;
pub mod lockfile;
pub mod lsp;
pub mod metadata;
pub mod migrations;
pub mod module;
pub mod operation;
pub mod os;
pub mod package;
pub mod path;
pub mod placeholder;
pub mod process;
pub mod pull;
pub mod push;
pub mod reference;
pub mod server;
pub mod symlink;
pub mod system;
pub mod template;
pub mod value;

pub struct Cli {
	/// The directory where the CLI stores its data.
	path: os::PathBuf,

	/// The lock used to acquire shared and exclusive access to the path.
	lock: Lock<()>,

	/// The database used to store artifacts, packages, and operations.
	database: Database,

	/// The semaphore used to prevent the CLI from opening too many files simultaneously.
	file_semaphore: Arc<Semaphore>,

	/// The semaphore used to prevent the CLI from opening too many sockets simultaneously.
	socket_semaphore: Arc<Semaphore>,

	/// The HTTP client for download operations.
	http_client: reqwest::Client,

	/// A handle to the tokio runtime the CLI was created on.
	runtime_handle: tokio::runtime::Handle,

	/// A local pool for running `!Send` futures.
	local_pool_handle: LocalPoolHandle,

	/// The channel to send requests to the language service.
	language_service_request_sender: std::sync::Mutex<Option<RequestSender>>,

	/// A map that tracks documents.
	documents:
		tokio::sync::RwLock<HashMap<module::Identifier, module::Document, fnv::FnvBuildHasher>>,

	/// A map that tracks changes to modules.
	module_trackers:
		tokio::sync::RwLock<HashMap<module::Identifier, module::Tracker, fnv::FnvBuildHasher>>,

	/// A client for communicating with the API.
	api_client: api::Client,
}

static V8_INIT: std::sync::Once = std::sync::Once::new();

fn initialize_v8() {
	// Set the ICU data.
	#[repr(C, align(16))]
	struct IcuData([u8; 10_541_264]);
	static ICU_DATA: IcuData = IcuData(*include_bytes!("../assets/icudtl.dat"));
	v8::icu::set_common_data_72(&ICU_DATA.0).unwrap();

	// Initialize the platform.
	let platform = v8::new_default_platform(0, true);
	v8::V8::initialize_platform(platform.make_shared());

	// Initialize V8.
	v8::V8::initialize();
}

impl Cli {
	pub async fn new(path: os::PathBuf) -> Result<Cli> {
		// Initialize V8.
		V8_INIT.call_once(initialize_v8);

		// Create the lock.
		let lock_path = path.join("lock");
		let lock = Lock::new(&lock_path, ());

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

		// Create the documents map.
		let documents = tokio::sync::RwLock::new(HashMap::default());

		// Create the module trackers map.
		let module_trackers = tokio::sync::RwLock::new(HashMap::default());

		// Create the API Client.
		let api_client = api::Client::new(api_url, token);

		// Create the CLI.
		let cli = Cli {
			path,
			lock,
			database,
			file_semaphore,
			socket_semaphore,
			http_client,
			runtime_handle,
			local_pool_handle,
			language_service_request_sender,
			documents,
			module_trackers,
			api_client,
		};

		Ok(cli)
	}
}

impl Cli {
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

impl Cli {
	#[must_use]
	fn path(&self) -> &os::Path {
		&self.path
	}

	#[must_use]
	fn blobs_path(&self) -> os::PathBuf {
		self.path().join("blobs")
	}

	#[must_use]
	fn checkouts_path(&self) -> os::PathBuf {
		self.path().join("checkouts")
	}

	#[must_use]
	fn config_path(&self) -> os::PathBuf {
		self.path().join("config.json")
	}

	#[must_use]
	fn credentials_path(&self) -> os::PathBuf {
		self.path().join("credentials.json")
	}

	#[must_use]
	fn temps_path(&self) -> os::PathBuf {
		self.path().join("temps")
	}

	#[must_use]
	fn blob_path(&self, blob_hash: blob::Hash) -> os::PathBuf {
		self.blobs_path().join(blob_hash.to_string())
	}

	#[must_use]
	fn temp_path(&self) -> os::PathBuf {
		self.temps_path().join(Id::generate().to_string())
	}
}

impl Cli {
	pub fn create_default_context(&self) -> Result<BTreeMap<String, Value>> {
		let host = System::host()?;
		let host = Value::String(host.to_string());
		let context = [("host".to_owned(), host)].into();
		Ok(context)
	}
}
