use self::lock::Lock;
use crate::{
	api,
	artifact::{self, Artifact},
	blob,
	client::Client,
	database::Database,
	document,
	error::Result,
	hash, language, operation,
	package::{self, Package},
	util::{fs, task_map::TaskMap},
	value::Value,
};
use std::{collections::HashMap, sync::Arc};
use tokio_util::task::LocalPoolHandle;
use url::Url;

mod clean;
mod lock;

/// An instance.
pub struct Instance {
	/// A client for communicating with the API.
	pub(crate) api_client: api::Client,

	/// The database.
	pub(crate) database: Database,

	/// A map of paths to documents.
	pub(crate) documents:
		tokio::sync::RwLock<HashMap<document::Document, document::State, fnv::FnvBuildHasher>>,

	/// An HTTP client for download operations.
	pub(crate) http_client: reqwest::Client,

	/// A task map that deduplicates internal checkouts.
	#[allow(clippy::type_complexity)]
	pub(crate) internal_checkouts_task_map:
		std::sync::Mutex<Option<Arc<TaskMap<Artifact, Result<fs::PathBuf>>>>>,

	/// A channel sender to send requests to the language service.
	pub(crate) language_service_request_sender:
		std::sync::Mutex<Option<language::service::RequestSender>>,

	/// A local pool for running `!Send` futures.
	pub(crate) local_pool_handle: LocalPoolHandle,

	/// A lock used to acquire shared and exclusive access to the path.
	pub(crate) lock: Lock<()>,

	/// A task map that deduplicates operations.
	#[allow(clippy::type_complexity)]
	pub(crate) operations_task_map:
		std::sync::Mutex<Option<Arc<TaskMap<operation::Hash, Result<Value>>>>>,

	/// A map of package specifiers to packages.
	pub(crate) packages: std::sync::RwLock<HashMap<Package, package::Specifier, hash::BuildHasher>>,

	/// The path to the directory where the instance stores its data.
	pub(crate) path: fs::PathBuf,

	/// A handle to the main tokio runtime.
	pub(crate) runtime_handle: tokio::runtime::Handle,
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

		// Ensure the path exists.
		tokio::fs::create_dir_all(&path).await?;

		// Migrate the path.
		Self::migrate(&path).await?;

		// Create the API Client.
		let api_url = options
			.api_url
			.unwrap_or_else(|| "https://api.tangram.dev".parse().unwrap());
		let token = options.api_token;
		let api_client = api::Client::new(api_url, token);

		// Open the database.
		let database_path = path.join("database.mdb");
		let database = Database::open(&database_path)?;

		// Create the documents maps.
		let documents = tokio::sync::RwLock::new(HashMap::default());

		// Create the HTTP client.
		let http_client = reqwest::Client::new();

		// Create the internal checkouts task map.
		let internal_checkouts_task_map = std::sync::Mutex::new(None);

		// Create the language service request sender.
		let language_service_request_sender = std::sync::Mutex::new(None);

		// Create the local pool handle.
		let threads = std::thread::available_parallelism().unwrap().get();
		let local_pool_handle = LocalPoolHandle::new(threads);

		// Create the lock.
		let lock_path = path.join("lock");
		let lock = Lock::new(&lock_path, ());

		// Create the operations task map.
		let operations_task_map = std::sync::Mutex::new(None);

		// Create the packages map.
		let packages = std::sync::RwLock::new(HashMap::default());

		// Get a handle to the tokio runtime.
		let runtime_handle = tokio::runtime::Handle::current();

		// Create the instance.
		let instance = Instance {
			api_client,
			database,
			documents,
			http_client,
			internal_checkouts_task_map,
			language_service_request_sender,
			local_pool_handle,
			lock,
			operations_task_map,
			packages,
			path,
			runtime_handle,
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

	pub fn api_instance_client(&self) -> &Client {
		self.api_client.instance_client()
	}
}
