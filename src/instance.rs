#[cfg(feature = "build")]
use crate::value::Value;
use crate::{error::Result, id::Id, util::task_map::TaskMap};
use derive_more::Deref;
#[cfg(feature = "language")]
use std::collections::HashMap;
use std::{
	path::{Path, PathBuf},
	sync::Arc,
};
use url::Url;

/// An instance.
#[derive(Clone, Deref)]
pub struct Instance {
	pub(crate) state: Arc<State>,
}

pub struct State {
	/// A semaphore that limits the number of concurrent commands.
	#[cfg(feature = "build")]
	pub(crate) command_semaphore: tokio::sync::Semaphore,

	/// The database.
	pub(crate) database: Database,

	/// A map of paths to documents.
	#[cfg(feature = "language")]
	pub(crate) documents:
		tokio::sync::RwLock<HashMap<Document, document::State, fnv::FnvBuildHasher>>,

	/// A semaphore that prevents opening too many file descriptors.
	pub(crate) file_descriptor_semaphore: tokio::sync::Semaphore,

	/// An HTTP client for downloading resources.
	#[cfg(feature = "build")]
	pub(crate) http_client: reqwest::Client,

	/// A task map that deduplicates internal checkouts.
	#[allow(clippy::type_complexity)]
	pub(crate) internal_checkouts_task_map:
		std::sync::Mutex<Option<Arc<TaskMap<Id, Result<PathBuf>>>>>,

	/// A channel sender to send requests to the language service.
	#[cfg(feature = "language")]
	pub(crate) language_service_request_sender:
		std::sync::Mutex<Option<crate::language::service::RequestSender>>,

	/// A local pool for running `!Send` futures.
	#[cfg(feature = "build")]
	pub(crate) local_pool: tokio_util::task::LocalPoolHandle,

	/// A handle to the main tokio runtime.
	#[cfg(feature = "language")]
	pub(crate) main_runtime_handle: tokio::runtime::Handle,

	/// A map that deduplicates runs.
	#[allow(clippy::type_complexity)]
	#[cfg(feature = "build")]
	pub(crate) operations_task_map: std::sync::Mutex<Option<Arc<TaskMap<Id, Result<Value>>>>>,

	/// The options the instance was created with.
	pub(crate) options: Options,

	/// A client for communicating with the parent.
	#[cfg(feature = "client")]
	pub(crate) parent: Client,

	/// The path to the directory where the instance stores its data.
	pub(crate) path: PathBuf,
}

pub(crate) struct Database {
	pub(crate) env: lmdb::Environment,
	pub(crate) values: lmdb::Database,
	pub(crate) runs: lmdb::Database,
	pub(crate) outputs: lmdb::Database,
}

#[derive(Clone, Debug, Default)]
pub struct Options {
	pub origin_token: Option<String>,
	pub origin_url: Option<Url>,
	pub preserve_temps: bool,
	pub sandbox_enabled: bool,
}

impl Instance {
	pub async fn new(path: PathBuf, options: Options) -> Result<Instance> {
		// Ensure the path exists.
		tokio::fs::create_dir_all(&path).await?;

		// Migrate the path.
		Self::migrate(&path).await?;

		#[cfg(feature = "build")]
		{
			// Initialize v8.
			V8_INIT.call_once(initialize_v8);
		}

		// Create the command semaphore.
		#[cfg(feature = "build")]
		let command_semaphore = tokio::sync::Semaphore::new(16);

		// Create the database.
		let database_path = path.join("database");
		let mut env_builder = lmdb::Environment::new();
		env_builder.set_map_size(1_099_511_627_776);
		env_builder.set_max_dbs(3);
		env_builder.set_max_readers(1024);
		env_builder.set_flags(lmdb::EnvironmentFlags::NO_SUB_DIR);
		let env = env_builder.open(&database_path)?;
		let values = env.open_db(Some("values"))?;
		let runs = env.open_db(Some("runs"))?;
		let outputs = env.open_db(Some("outputs"))?;
		let database = Database {
			env,
			values,
			runs,
			outputs,
		};

		// Create the documents maps.
		#[cfg(feature = "language")]
		let documents = tokio::sync::RwLock::new(HashMap::default());

		// Create the file system semaphore.
		let file_descriptor_semaphore = tokio::sync::Semaphore::new(16);

		// Create the HTTP client.
		#[cfg(feature = "build")]
		let http_client = reqwest::Client::new();

		// Create the internal checkouts task map.
		let internal_checkouts_task_map = std::sync::Mutex::new(None);

		// Create the sender for language service requests.
		#[cfg(feature = "language")]
		let language_service_request_sender = std::sync::Mutex::new(None);

		// Create the local pool handle.
		#[cfg(feature = "build")]
		let local_pool = tokio_util::task::LocalPoolHandle::new(
			std::thread::available_parallelism().unwrap().get(),
		);

		// Get the curent tokio runtime handler.
		#[cfg(feature = "language")]
		let main_runtime_handle = tokio::runtime::Handle::current();

		// Create the operations task map.
		#[cfg(feature = "build")]
		let operations_task_map = std::sync::Mutex::new(None);

		// Create the parent client.
		#[cfg(feature = "client")]
		let parent = {
			let url = options
				.origin_url
				.clone()
				.unwrap_or_else(|| API_URL.parse().unwrap());
			let token = options.origin_token.clone();
			Client::new(url, token)
		};

		// Create the state.
		let state = State {
			#[cfg(feature = "build")]
			command_semaphore,
			database,
			#[cfg(feature = "language")]
			documents,
			file_descriptor_semaphore,
			#[cfg(feature = "build")]
			http_client,
			internal_checkouts_task_map,
			#[cfg(feature = "language")]
			language_service_request_sender,
			#[cfg(feature = "build")]
			local_pool,
			#[cfg(feature = "language")]
			main_runtime_handle,
			#[cfg(feature = "build")]
			operations_task_map,
			options,
			#[cfg(feature = "client")]
			parent,
			path,
		};

		// Create the instance.
		let instance = Instance {
			state: Arc::new(state),
		};

		Ok(instance)
	}
}

#[cfg(feature = "build")]
static V8_INIT: std::sync::Once = std::sync::Once::new();

#[cfg(feature = "build")]
fn initialize_v8() {
	// Set the ICU data.
	#[repr(C, align(16))]
	struct IcuData([u8; 10_631_872]);
	static ICU_DATA: IcuData = IcuData(*include_bytes!(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/assets/icudtl.dat"
	)));
	v8::icu::set_common_data_73(&ICU_DATA.0).unwrap();

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

	// #[must_use]
	// pub fn parent(&self) -> &Client {
	// 	&self.parent
	// }
}
