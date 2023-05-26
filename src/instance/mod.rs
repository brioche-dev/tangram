use self::lock::Lock;
use crate::{
	api,
	artifact::{self, Artifact},
	blob,
	client::Client,
	database::Database,
	document,
	error::Result,
	language, operation,
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

#[cfg(feature = "v8")]
pub(crate) mod language;

#[cfg(feature = "v8")]
pub(crate) mod operations;
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
		std::sync::Mutex<Option<Arc<TaskMap<Artifact, Result<PathBuf>>>>>,

	/// A lock used to acquire shared and exclusive access to the path.
	pub(crate) lock: Lock<()>,

	/// The path to the directory where the instance stores its data.
	pub(crate) path: PathBuf,

	/// A map of package specifiers to packages.
	pub(crate) packages: std::sync::RwLock<HashMap<Package, package::Specifier, hash::BuildHasher>>,

	/// The path to the directory where the instance stores its data.
	pub(crate) path: PathBuf,

	/// State required to provide support for running operations.
	#[cfg(feature = "run")]
	pub(crate) operations: operations::State,
}

#[derive(Clone, Debug, Default)]
pub struct Options {
	pub api_url: Option<Url>,
	pub api_token: Option<String>,
}

impl Instance {
	pub async fn new(path: PathBuf, options: Options) -> Result<Instance> {
		// Create the API Client.
		let api_url = options
			.api_url
			.unwrap_or_else(|| "https://api.tangram.dev".parse().unwrap());
		let token = options.api_token;
		let api_client = api::Client::new(api_url, token);

		// Create the documents maps.
		let documents = tokio::sync::RwLock::new(HashMap::default());

		// Open the database.
		let database_path = path.join("database.mdb");
		let database = Database::open(&database_path)?;

		// Create the documents maps.
		let documents = tokio::sync::RwLock::new(HashMap::default());

		// Create the HTTP client.
		let http_client = reqwest::Client::new();

		// Create the internal checkouts task map.
		let internal_checkouts_task_map = std::sync::Mutex::new(None);

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
