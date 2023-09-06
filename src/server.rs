use crate::{
	client::API_URL,
	document::{self, Document},
	error::Result,
	id::Id,
	// language,
	value,
	Client,
	Error,
};
use async_recursion::async_recursion;
use futures::{StreamExt, TryStreamExt};
use lmdb::Transaction;
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::Arc,
};
use url::Url;

/// A server.
#[derive(Clone, Debug)]
pub struct Server {
	pub(crate) state: Arc<State>,
}

#[derive(Debug)]
pub struct State {
	/// A semaphore that limits the number of concurrent commands.
	pub(crate) command_semaphore: tokio::sync::Semaphore,

	/// The database.
	pub(crate) database: Database,

	/// A map of paths to documents.
	pub(crate) documents:
		tokio::sync::RwLock<HashMap<Document, document::State, fnv::FnvBuildHasher>>,

	/// A semaphore that prevents opening too many file descriptors.
	pub(crate) file_descriptor_semaphore: tokio::sync::Semaphore,

	/// An HTTP client for downloading resources.
	pub(crate) http_client: reqwest::Client,

	/// A channel sender to send requests to the language service.
	// pub(crate) language_service_request_sender:
	// 	std::sync::Mutex<Option<language::service::RequestSender>>,

	/// A local pool for running `!Send` futures.
	pub(crate) local_pool: tokio_util::task::LocalPoolHandle,

	/// A handle to the main tokio runtime.
	pub(crate) main_runtime_handle: tokio::runtime::Handle,

	/// The options the server was created with.
	pub(crate) options: Options,

	/// A client for communicating with the parent.
	pub(crate) parent: Client,

	/// The path to the directory where the server stores its data.
	pub(crate) path: PathBuf,
}

#[derive(Debug)]
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

impl Server {
	pub async fn new(path: PathBuf, options: Options) -> Result<Server> {
		// Ensure the path exists.
		tokio::fs::create_dir_all(&path).await?;

		// Migrate the path.
		Self::migrate(&path).await?;

		// Initialize v8.
		V8_INIT.call_once(initialize_v8);

		// Create the command semaphore.
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
		let documents = tokio::sync::RwLock::new(HashMap::default());

		// Create the file system semaphore.
		let file_descriptor_semaphore = tokio::sync::Semaphore::new(16);

		// Create the HTTP client.
		let http_client = reqwest::Client::new();

		// Create the sender for language service requests.
		// let language_service_request_sender = std::sync::Mutex::new(None);

		// Create the local pool handle.
		let local_pool = tokio_util::task::LocalPoolHandle::new(
			std::thread::available_parallelism().unwrap().get(),
		);

		// Get the curent tokio runtime handler.
		let main_runtime_handle = tokio::runtime::Handle::current();

		// Create the parent client.
		let parent = {
			let url = options
				.origin_url
				.clone()
				.unwrap_or_else(|| API_URL.parse().unwrap());
			let token = options.origin_token.clone();
			Client::new_remote(url, token)
		};

		// Create the state.
		let state = State {
			command_semaphore,
			database,
			documents,
			file_descriptor_semaphore,
			http_client,
			// language_service_request_sender,
			local_pool,
			main_runtime_handle,
			options,
			parent,
			path,
		};

		// Create the server.
		let server = Server {
			state: Arc::new(state),
		};

		Ok(server)
	}
}

static V8_INIT: std::sync::Once = std::sync::Once::new();

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

impl Server {
	#[must_use]
	pub fn path(&self) -> &Path {
		&self.state.path
	}

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
	pub fn parent(&self) -> &Client {
		&self.state.parent
	}

	#[async_recursion]
	pub async fn value_exists(&self, id: Id) -> Result<bool> {
		// Check if the value exists locally.
		{
			let txn = self.state.database.env.begin_ro_txn()?;
			match txn.get(self.state.database.values, &id.as_bytes()) {
				Ok(_) => return Ok(true),
				Err(lmdb::Error::NotFound) => {},
				Err(error) => return Err(error.into()),
			};
		}

		// Check if the value exists remotely.
		{
			if self.state.parent.value_exists(id).await? {
				return Ok(true);
			}
		}

		Ok(false)
	}

	#[async_recursion]
	pub async fn try_get_value_bytes(&self, id: Id) -> Result<Option<Vec<u8>>> {
		// Attempt to get the value locally.
		{
			let txn = self.state.database.env.begin_ro_txn()?;
			let data = match txn.get(self.state.database.values, &id.as_bytes()) {
				Ok(data) => return Ok(Some(data.to_owned())),
				Err(lmdb::Error::NotFound) => {},
				Err(error) => return Err(error.into()),
			};
		}

		// Attempt to get the value remotely.
		if let Some(bytes) = self.state.parent.try_get_value_bytes(id).await? {
			// Create a write transaction.
			let mut txn = self.state.database.env.begin_rw_txn()?;

			// Add the value to the database.
			txn.put(
				self.state.database.values,
				&id.as_bytes(),
				&bytes,
				lmdb::WriteFlags::empty(),
			)?;

			// Commit the transaction.
			txn.commit()?;

			return Ok(Some(bytes));
		}

		Ok(None)
	}

	pub async fn try_put_value_bytes(&self, id: Id, bytes: &[u8]) -> Result<Result<(), Vec<Id>>> {
		// Deserialize the bytes.
		let data = value::Data::deserialize(bytes)?;

		// Check if there are any missing children.
		let missing_children = futures::stream::iter(data.children())
			.map(Ok)
			.try_filter_map(|id| async move {
				let exists = self.value_exists(id).await?;
				Ok::<_, Error>(if exists { None } else { Some(id) })
			})
			.try_collect::<Vec<_>>()
			.await?;
		if !missing_children.is_empty() {
			return Ok(Err(missing_children));
		}

		// Create a write transaction.
		let mut txn = self.state.database.env.begin_rw_txn()?;

		// Serialize the data.
		let bytes = data.serialize()?;

		// Add the value to the database.
		txn.put(
			self.state.database.values,
			&id.as_bytes(),
			&bytes,
			lmdb::WriteFlags::empty(),
		)?;

		// Commit the transaction.
		txn.commit()?;

		Ok(Ok(()))
	}
}
