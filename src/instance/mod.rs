use self::lock::Lock;
#[cfg(feature = "operation_run")]
use crate::value::Value;
use crate::{
	api,
	artifact::{self, Artifact},
	blob,
	client::Client,
	database::Database,
	document,
	error::Result,
	operation,
	util::task_map::TaskMap,
};
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::Arc,
};
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

	/// A semaphore that prevents opening too many file descriptors.
	pub(crate) file_descriptor_semaphore: tokio::sync::Semaphore,

	/// An HTTP client for downloading resources.
	#[cfg(feature = "operation_run")]
	pub(crate) http_client: reqwest::Client,

	/// A task map that deduplicates internal checkouts.
	#[allow(clippy::type_complexity)]
	pub(crate) internal_checkouts_task_map:
		std::sync::Mutex<Option<Arc<TaskMap<Artifact, Result<PathBuf>>>>>,

	/// A channel sender to send requests to the language service.
	#[cfg(feature = "language")]
	pub(crate) language_service_request_sender:
		std::sync::Mutex<Option<crate::language::service::RequestSender>>,

	/// A local pool for running `!Send` futures.
	#[cfg(feature = "operation_run")]
	pub(crate) local_pool: tokio_util::task::LocalPoolHandle,

	/// A lock used to acquire shared and exclusive access to the path.
	pub(crate) lock: Lock<()>,

	/// A handle to the main tokio runtime.
	#[cfg(feature = "language")]
	pub(crate) main_runtime_handle: tokio::runtime::Handle,

	/// A task map that deduplicates operations.
	#[allow(clippy::type_complexity)]
	#[cfg(feature = "operation_run")]
	pub(crate) operations_task_map:
		std::sync::Mutex<Option<Arc<TaskMap<operation::Hash, Result<Value>>>>>,

	#[cfg(feature = "operation_run")]
	pub(crate) process_semaphore: tokio::sync::Semaphore,

	/// The path to the directory where the instance stores its data.
	pub(crate) path: PathBuf,

	/// The configuration options for creating the instance.
	pub(crate) options: Options,
}

#[derive(Clone, Debug, Default)]
pub struct Options {
	pub api_url: Option<Url>,
	pub api_token: Option<String>,
	pub sandbox_enabled: bool,
}

impl Instance {
	pub async fn new(path: PathBuf, options: Options) -> Result<Instance> {
		// Ensure the path exists.
		tokio::fs::create_dir_all(&path).await?;

		// Migrate the path.
		Self::migrate(&path).await?;

		#[cfg(feature = "v8")]
		{
			// Initialize v8.
			V8_INIT.call_once(initialize_v8);
		}

		// Create the API Client.
		let api_url = options
			.api_url
			.clone()
			.unwrap_or_else(|| "https://api.tangram.dev".parse().unwrap());
		let token = options.api_token.clone();
		let api_client = api::Client::new(api_url, token);

		// Create the documents maps.
		let documents = tokio::sync::RwLock::new(HashMap::default());

		// Create the file system semaphore.
		let file_descriptor_semaphore = tokio::sync::Semaphore::new(16);

		// Open the database.
		let database_path = path.join("database.mdb");
		let database = Database::open(&database_path)?;

		// Create the HTTP client.
		#[cfg(feature = "operation_run")]
		let http_client = reqwest::Client::new();

		// Create the internal checkouts task map.
		let internal_checkouts_task_map = std::sync::Mutex::new(None);

		// Create a new sender for the service request.
		#[cfg(feature = "language")]
		let language_service_request_sender = std::sync::Mutex::new(None);

		// Create the local pool handle.
		#[cfg(feature = "operation_run")]
		let local_pool = tokio_util::task::LocalPoolHandle::new(
			std::thread::available_parallelism().unwrap().get(),
		);

		// Create the lock.
		let lock_path = path.join("lock");
		let lock = Lock::new(&lock_path, ());

		// Get the curent tokio runtime handler.
		#[cfg(feature = "language")]
		let main_runtime_handle = tokio::runtime::Handle::current();

		// Create the operations task map.
		#[cfg(feature = "operation_run")]
		let operations_task_map = std::sync::Mutex::new(None);

		// Create the process semaphore.
		#[cfg(feature = "operation_run")]
		let process_semaphore = tokio::sync::Semaphore::new(16);

		// Create the instance.
		let instance = Instance {
			api_client,
			database,
			documents,
			file_descriptor_semaphore,
			#[cfg(feature = "operation_run")]
			http_client,
			internal_checkouts_task_map,
			#[cfg(feature = "language")]
			language_service_request_sender,
			#[cfg(feature = "operation_run")]
			local_pool,
			lock,
			#[cfg(feature = "language")]
			main_runtime_handle,
			#[cfg(feature = "operation_run")]
			operations_task_map,
			#[cfg(feature = "operation_run")]
			process_semaphore,
			path,
			options,
		};

		Ok(instance)
	}
}

#[cfg(feature = "v8")]
static V8_INIT: std::sync::Once = std::sync::Once::new();

#[cfg(feature = "v8")]
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
	pub fn path(&self) -> &Path {
		&self.path
	}

	#[must_use]
	pub fn artifacts_path(&self) -> PathBuf {
		self.path().join("artifacts")
	}

	#[must_use]
	pub fn artifact_path(&self, artifact_hash: artifact::Hash) -> PathBuf {
		self.artifacts_path().join(artifact_hash.to_string())
	}

	#[must_use]
	pub fn assets_path(&self) -> PathBuf {
		self.path().join("assets")
	}

	#[must_use]
	pub fn blobs_path(&self) -> PathBuf {
		self.path().join("blobs")
	}

	#[must_use]
	pub fn blob_path(&self, blob_hash: blob::Hash) -> PathBuf {
		self.blobs_path().join(blob_hash.to_string())
	}

	#[must_use]
	pub fn database_path(&self) -> PathBuf {
		self.path().join("database.mdb")
	}

	#[must_use]
	pub fn logs_path(&self) -> PathBuf {
		self.path().join("logs")
	}

	#[must_use]
	pub fn log_path(&self, operation_hash: operation::Hash) -> PathBuf {
		self.logs_path().join(operation_hash.to_string())
	}

	#[must_use]
	pub fn temps_path(&self) -> PathBuf {
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
