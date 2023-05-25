use self::lock::Lock;
use crate::{
	api,
	artifact::{self, Artifact},
	blob,
	client::Client,
	database::Database,
	error::Result,
	operation,
	util::task_map::TaskMap,
};
use std::{
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

#[cfg(feature = "modules")]
pub(crate) mod module;

/// An instance.
pub struct Instance {
	/// A client for communicating with the API.
	pub(crate) api_client: api::Client,

	/// The database.
	pub(crate) database: Database,

	/// A task map that deduplicates internal checkouts.
	#[allow(clippy::type_complexity)]
	pub(crate) internal_checkouts_task_map:
		std::sync::Mutex<Option<Arc<TaskMap<Artifact, Result<PathBuf>>>>>,

	/// A lock used to acquire shared and exclusive access to the path.
	pub(crate) lock: Lock<()>,

	/// The path to the directory where the instance stores its data.
	pub(crate) path: PathBuf,

	/// State required to provide modules support.
	#[cfg(feature = "modules")]
	pub(crate) modules: module::State,

	/// State required to provide language support to an instance.
	#[cfg(feature = "v8")]
	pub(crate) language: language::State,

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

		// Create the internal checkouts task map.
		let internal_checkouts_task_map = std::sync::Mutex::new(None);

		// Create the lock.
		let lock_path = path.join("lock");
		let lock = Lock::new(&lock_path, ());

		// Create the state required for module support.
		#[cfg(feature = "modules")]
		let modules = module::State::new();

		// Create the language handles.
		#[cfg(feature = "v8")]
		let language = language::State::new();

		// Create the state needed to run operations.
		#[cfg(feature = "run")]
		let run = operations::State::new();

		// Create the instance.
		let instance = Instance {
			api_client,
			database,
			internal_checkouts_task_map,
			lock,
			path,

			#[cfg(feature = "modules")]
			modules,

			#[cfg(feature = "v8")]
			language,

			#[cfg(feature = "run")]
			operations: run,
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
