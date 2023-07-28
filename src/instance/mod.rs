use crate::{
	artifact::Artifact,
	block::Block,
	client::{Client, API_URL},
	document::{self, Document},
	error::Result,
	util::task_map::TaskMap,
	value::Value,
};
use derive_more::Deref;
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::Arc,
};
use url::Url;

mod clean;

/// An instance.
#[derive(Clone, Deref)]
pub struct Instance {
	pub(crate) state: Arc<State>,
}

pub struct State {
	/// A client for communicating with the API.
	pub(crate) api_client: Client,

	/// The database connection pool.
	pub(crate) database_connection_pool: deadpool_sqlite::Pool,

	/// A map of paths to documents.
	pub(crate) documents:
		tokio::sync::RwLock<HashMap<Document, document::State, fnv::FnvBuildHasher>>,

	/// A semaphore that prevents opening too many file descriptors.
	pub(crate) file_descriptor_semaphore: tokio::sync::Semaphore,

	/// An HTTP client for downloading resources.
	#[cfg(feature = "evaluate")]
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
	#[cfg(feature = "evaluate")]
	pub(crate) local_pool: tokio_util::task::LocalPoolHandle,

	/// A handle to the main tokio runtime.
	#[cfg(feature = "language")]
	pub(crate) main_runtime_handle: tokio::runtime::Handle,

	/// A task map that deduplicates operations.
	#[allow(clippy::type_complexity)]
	#[cfg(feature = "evaluate")]
	pub(crate) operations_task_map: std::sync::Mutex<Option<Arc<TaskMap<Block, Result<Value>>>>>,

	/// The options the instance was created with.
	#[cfg(feature = "evaluate")]
	pub(crate) options: Options,

	/// The path to the directory where the instance stores its data.
	pub(crate) path: PathBuf,

	/// Whether to preserve temporary files.
	pub preserve_temps: bool,

	/// A semaphore that prevents running too many processes.
	#[cfg(feature = "evaluate")]
	pub(crate) process_semaphore: tokio::sync::Semaphore,
}

#[derive(Clone, Debug, Default)]
pub struct Options {
	pub api_url: Option<Url>,
	pub api_token: Option<String>,
	pub preserve_temps: bool,
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
			.unwrap_or_else(|| API_URL.parse().unwrap());
		let token = options.api_token.clone();
		let api_client = Client::new(api_url, token);

		// Create the documents maps.
		let documents = tokio::sync::RwLock::new(HashMap::default());

		// Create the file system semaphore.
		let file_descriptor_semaphore = tokio::sync::Semaphore::new(16);

		// Create the database pool.
		let database_path = path.join("database");
		let database_connection_pool = deadpool_sqlite::Config::new(database_path)
			.builder(deadpool_sqlite::Runtime::Tokio1)
			.unwrap()
			.max_size(std::thread::available_parallelism().unwrap().get())
			.build()
			.unwrap();

		// Create the HTTP client.
		#[cfg(feature = "evaluate")]
		let http_client = reqwest::Client::new();

		// Create the internal checkouts task map.
		let internal_checkouts_task_map = std::sync::Mutex::new(None);

		// Create a new sender for the service request.
		#[cfg(feature = "language")]
		let language_service_request_sender = std::sync::Mutex::new(None);

		// Create the local pool handle.
		#[cfg(feature = "evaluate")]
		let local_pool = tokio_util::task::LocalPoolHandle::new(
			std::thread::available_parallelism().unwrap().get(),
		);

		// Get the curent tokio runtime handler.
		#[cfg(feature = "language")]
		let main_runtime_handle = tokio::runtime::Handle::current();

		// Create the operations task map.
		#[cfg(feature = "evaluate")]
		let operations_task_map = std::sync::Mutex::new(None);

		// Store the option for preserving temps.
		let preserve_temps = options.preserve_temps;

		// Create the process semaphore.
		#[cfg(feature = "evaluate")]
		let process_semaphore = tokio::sync::Semaphore::new(16);

		// Create the state.
		let state = State {
			api_client,
			database_connection_pool,
			documents,
			file_descriptor_semaphore,
			#[cfg(feature = "evaluate")]
			http_client,
			internal_checkouts_task_map,
			#[cfg(feature = "language")]
			language_service_request_sender,
			#[cfg(feature = "evaluate")]
			local_pool,
			#[cfg(feature = "language")]
			main_runtime_handle,
			#[cfg(feature = "evaluate")]
			operations_task_map,
			#[cfg(feature = "evaluate")]
			options,
			path,
			preserve_temps,
			#[cfg(feature = "evaluate")]
			process_semaphore,
		};

		// Create the instance.
		let instance = Instance {
			state: Arc::new(state),
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
	#[must_use]
	pub fn path(&self) -> &Path {
		&self.state.path
	}
}

impl Instance {
	#[must_use]
	pub fn artifacts_path(&self) -> PathBuf {
		self.path().join("artifacts")
	}

	#[must_use]
	pub fn database_path(&self) -> PathBuf {
		self.path().join("database")
	}

	#[must_use]
	pub fn temps_path(&self) -> PathBuf {
		self.path().join("temps")
	}

	#[must_use]
	pub fn api_client(&self) -> &Client {
		&self.api_client
	}
}
