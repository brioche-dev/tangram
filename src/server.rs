use crate::{
	document::{self, Document},
	id, language,
	run::{self},
	value, Blob, Client, Error, Evaluation, Result, WrapErr,
};
use futures::{
	stream::{self, BoxStream},
	StreamExt, TryStreamExt,
};
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
	/// The database.
	pub(crate) database: Database,

	/// A map of paths to documents.
	pub(crate) documents:
		tokio::sync::RwLock<HashMap<Document, document::State, fnv::FnvBuildHasher>>,

	/// A semaphore that prevents opening too many file descriptors.
	pub(crate) file_descriptor_semaphore: tokio::sync::Semaphore,

	/// A channel sender to send requests to the language service.
	pub(crate) language_service_request_sender:
		std::sync::Mutex<Option<language::service::RequestSender>>,

	/// A handle to the main tokio runtime.
	pub(crate) main_runtime_handle: tokio::runtime::Handle,

	/// The options the server was created with.
	pub(crate) options: Options,

	/// A client for communicating with the parent.
	pub(crate) parent: Option<Client>,

	/// The path to the directory where the server stores its data.
	pub(crate) path: PathBuf,

	/// An HTTP client for downloading resources.
	pub(crate) resource_http_client: reqwest::Client,

	/// The running evaluations.
	pub(crate) running_evaluations:
		std::sync::RwLock<HashMap<run::Id, Arc<run::State>, id::BuildHasher>>,

	/// A local pool for running targets.
	pub(crate) target_local_pool: tokio_util::task::LocalPoolHandle,

	/// A semaphore that limits the number of concurrent tasks.
	pub(crate) task_semaphore: tokio::sync::Semaphore,
}

#[derive(Debug)]
pub(crate) struct Database {
	pub(crate) env: lmdb::Environment,
	pub(crate) values: lmdb::Database,
	pub(crate) evaluations: lmdb::Database,
	pub(crate) assignments: lmdb::Database,
}

#[derive(Clone, Debug, Default)]
pub struct Options {
	pub parent_token: Option<String>,
	pub parent_url: Option<Url>,
}

impl Server {
	pub async fn new(path: PathBuf, options: Options) -> Result<Server> {
		// Ensure the path exists.
		tokio::fs::create_dir_all(&path).await?;

		// Migrate the path.
		Self::migrate(&path).await?;

		// Initialize v8.
		V8_INIT.call_once(initialize_v8);

		// Create the database.
		let database_path = path.join("database");
		let mut env_builder = lmdb::Environment::new();
		env_builder.set_map_size(1_099_511_627_776);
		env_builder.set_max_dbs(3);
		env_builder.set_max_readers(1024);
		env_builder.set_flags(lmdb::EnvironmentFlags::NO_SUB_DIR);
		let env = env_builder.open(&database_path)?;
		let values = env.open_db(Some("values"))?;
		let evaluations = env.open_db(Some("evaluations"))?;
		let assignments = env.open_db(Some("assignments"))?;
		let database = Database {
			env,
			values,
			evaluations,
			assignments,
		};

		// Create the documents maps.
		let documents = tokio::sync::RwLock::new(HashMap::default());

		// Create the file system semaphore.
		let file_descriptor_semaphore = tokio::sync::Semaphore::new(16);

		// Create the sender for language service requests.
		let language_service_request_sender = std::sync::Mutex::new(None);

		// Get the curent tokio runtime handler.
		let main_runtime_handle = tokio::runtime::Handle::current();

		// Create the parent client.
		let parent = if let Some(url) = options.parent_url.as_ref() {
			let token = options.parent_token.clone();
			Some(Client::with_url(url.clone(), token))
		} else {
			None
		};

		// Create the HTTP client for downloading resources.
		let resource_http_client = reqwest::Client::new();

		// Create the running evaluations.
		let running_evaluations = std::sync::RwLock::new(HashMap::default());

		// Create the target local pool.
		let target_local_pool = tokio_util::task::LocalPoolHandle::new(
			std::thread::available_parallelism().unwrap().get(),
		);

		// Create the task semaphore.
		let task_semaphore =
			tokio::sync::Semaphore::new(std::thread::available_parallelism().unwrap().get());

		// Create the state.
		let state = State {
			database,
			documents,
			file_descriptor_semaphore,
			language_service_request_sender,
			main_runtime_handle,
			options,
			parent,
			path,
			resource_http_client,
			running_evaluations,
			target_local_pool,
			task_semaphore,
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

	pub async fn get_value_exists(&self, id: crate::Id) -> Result<bool> {
		// Check if the value exists in the database.
		if self.get_value_exists_from_database(id)? {
			return Ok(true);
		}

		// Check if the value exists in the parent.
		if self.get_value_exists_from_parent(id).await? {
			return Ok(true);
		}

		Ok(false)
	}

	pub(crate) fn get_value_exists_from_database(&self, id: crate::Id) -> Result<bool> {
		let txn = self.state.database.env.begin_ro_txn()?;
		match txn.get(self.state.database.values, &id.as_bytes()) {
			Ok(_) => Ok(true),
			Err(lmdb::Error::NotFound) => Ok(false),
			Err(error) => Err(error.into()),
		}
	}

	async fn get_value_exists_from_parent(&self, id: crate::Id) -> Result<bool> {
		if let Some(parent) = self.state.parent.as_ref() {
			if parent.get_value_exists(id).await? {
				return Ok(true);
			}
		}
		Ok(false)
	}

	pub async fn get_value_bytes(&self, id: crate::Id) -> Result<Vec<u8>> {
		self.try_get_value_bytes(id)
			.await?
			.wrap_err("Failed to get the value.")
	}

	pub async fn try_get_value_bytes(&self, id: crate::Id) -> Result<Option<Vec<u8>>> {
		// Attempt to get the value from the database.
		if let Some(bytes) = self.try_get_value_bytes_from_database(id)? {
			return Ok(Some(bytes));
		};

		// Attempt to get the value from the parent.
		if let Some(bytes) = self.try_get_value_bytes_from_parent(id).await? {
			return Ok(Some(bytes));
		};

		Ok(None)
	}

	pub fn try_get_value_bytes_from_database(&self, id: crate::Id) -> Result<Option<Vec<u8>>> {
		let txn = self.state.database.env.begin_ro_txn()?;
		match txn.get(self.state.database.values, &id.as_bytes()) {
			Ok(data) => Ok(Some(data.to_owned())),
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => Err(error.into()),
		}
	}

	async fn try_get_value_bytes_from_parent(&self, id: crate::Id) -> Result<Option<Vec<u8>>> {
		let Some(parent) = self.state.parent.as_ref() else {
			return Ok(None);
		};

		// Get the value from the parent.
		let Some(bytes) = parent.try_get_value_bytes(id).await? else {
			return Ok(None);
		};

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

		Ok(Some(bytes))
	}

	/// Attempt to put a value.
	pub async fn try_put_value_bytes(
		&self,
		id: crate::Id,
		bytes: &[u8],
	) -> Result<Result<(), Vec<crate::Id>>> {
		// Deserialize the bytes.
		let data = value::Data::deserialize(bytes)?;

		// Check if there are any missing children.
		let missing_children = stream::iter(data.children())
			.map(Ok)
			.try_filter_map(|id| async move {
				let exists = self.get_value_exists(id).await?;
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

	/// Evaluate a value.
	pub async fn evaluate(&self, id: crate::Id) -> Result<run::Id> {
		// Create a new evaluation.
		let evaluation_id = todo!();
		let state = Arc::new(run::State::new()?);
		self.state
			.running_evaluations
			.write()
			.unwrap()
			.insert(evaluation_id, state);

		// Create a write transaction.
		let mut txn = self.state.database.env.begin_rw_txn()?;

		// Add the assignment to the database.
		txn.put(
			self.state.database.assignments,
			&id.as_bytes(),
			&evaluation_id.as_bytes(),
			lmdb::WriteFlags::empty(),
		)?;

		// Commit the transaction.
		txn.commit()?;

		Ok(evaluation_id)
	}

	pub async fn try_get_assignment(&self, id: crate::Id) -> Result<Option<run::Id>> {
		// Attempt to get the assignment from the database.
		if let Some(evaluation_id) = self.try_get_assignment_from_database(id)? {
			return Ok(Some(evaluation_id));
		}

		// Attempt to get the assignment from the parent.
		if let Some(evaluation_id) = self.try_get_assignment_from_parent(id).await? {
			return Ok(Some(evaluation_id));
		}

		Ok(None)
	}

	fn try_get_assignment_from_database(&self, id: crate::Id) -> Result<Option<run::Id>> {
		// Get the assignment from the database.
		let txn = self.state.database.env.begin_ro_txn()?;
		match txn.get(self.state.database.assignments, &id.as_bytes()) {
			Ok(evaluation_id) => Ok(Some(run::Id::with_bytes(
				evaluation_id
					.try_into()
					.map_err(Error::other)
					.wrap_err("Invalid ID.")?,
			)?)),
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => Err(error.into()),
		}
	}

	async fn try_get_assignment_from_parent(&self, id: crate::Id) -> Result<Option<run::Id>> {
		// Get the parent.
		let Some(parent) = self.state.parent.as_ref() else {
			return Ok(None);
		};

		// Get the assignment.
		let Some(evaluation_id) = parent.try_get_assignment(id).await? else {
			return Ok(None);
		};

		// Create a write transaction.
		let mut txn = self.state.database.env.begin_rw_txn()?;

		// Add the assignment to the database.
		txn.put(
			self.state.database.assignments,
			&id.as_bytes(),
			&evaluation_id.as_bytes(),
			lmdb::WriteFlags::empty(),
		)?;

		// Commit the transaction.
		txn.commit()?;

		Ok(Some(evaluation_id))
	}

	pub async fn try_get_evaluation(&self, id: run::Id) -> Result<Option<Evaluation>> {
		let Some(bytes) = self.try_get_evaluation_bytes(id).await? else {
			return Ok(None);
		};
		let evaluation = Evaluation::deserialize(&bytes)?;
		Ok(Some(evaluation))
	}

	pub async fn try_get_evaluation_bytes(&self, id: run::Id) -> Result<Option<Vec<u8>>> {
		// Attempt to get the evaluation from the database.
		if let Some(bytes) = self.try_get_evaluation_bytes_from_database(id)? {
			return Ok(Some(bytes));
		}

		// Attempt to get the evaluation from the parent.
		if let Some(bytes) = self.try_get_evaluation_bytes_from_parent(id).await? {
			return Ok(Some(bytes));
		}

		Ok(None)
	}

	fn try_get_evaluation_bytes_from_database(&self, id: run::Id) -> Result<Option<Vec<u8>>> {
		let txn = self.state.database.env.begin_ro_txn()?;
		match txn.get(self.state.database.evaluations, &id.as_bytes()) {
			Ok(evaluation) => Ok(Some(evaluation.to_owned())),
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => Err(error.into()),
		}
	}

	async fn try_get_evaluation_bytes_from_parent(&self, id: run::Id) -> Result<Option<Vec<u8>>> {
		let Some(parent) = self.state.parent.as_ref() else {
			return Ok(None);
		};

		// Get the evaluation from the parent.
		let Some(bytes) = parent.try_get_evaluation_bytes(id).await? else {
			return Ok(None);
		};

		// Create a write transaction.
		let mut txn = self.state.database.env.begin_rw_txn()?;

		// Add the evaluation to the database.
		txn.put(
			self.state.database.evaluations,
			&id.as_bytes(),
			&bytes,
			lmdb::WriteFlags::empty(),
		)?;

		// Commit the transaction.
		txn.commit()?;

		Ok(Some(bytes))
	}

	pub async fn try_get_evaluation_children(
		&self,
		id: run::Id,
	) -> Result<Option<BoxStream<'static, run::Id>>> {
		// Attempt to get the evaluation children from the running evaluations.
		'a: {
			let state = self
				.state
				.running_evaluations
				.read()
				.unwrap()
				.get(&id)
				.cloned();
			let Some(state) = state else {
				break 'a;
			};
			return Ok(Some(state.children()));
		}

		// Attempt to get the evaluation children from the database or the parent.
		'a: {
			let Some(evaluation) = self.try_get_evaluation(id).await? else {
				break 'a;
			};
			return Ok(Some(stream::iter(evaluation.children).boxed()));
		}

		Ok(None)
	}

	pub async fn try_get_evaluation_log(
		&self,
		id: run::Id,
	) -> Result<Option<BoxStream<'static, Vec<u8>>>> {
		// Attempt to get the evaluation log from the running evaluations.
		let state = self
			.state
			.running_evaluations
			.read()
			.unwrap()
			.get(&id)
			.cloned();
		if let Some(state) = state {
			let log = state.log().await?;
			return Ok(Some(log));
		}

		// Attempt to get the evaluation log from the database or the parent.
		'a: {
			let Some(evaluation) = self.try_get_evaluation(id).await? else {
				break 'a;
			};
			let client = &Client::with_server(self.clone());
			let blob = Blob::with_id(evaluation.log);
			let bytes = blob.bytes(client).await?;
			return Ok(Some(stream::once(async move { bytes }).boxed()));
		}

		Ok(None)
	}

	pub async fn try_get_evaluation_result(
		&self,
		id: run::Id,
	) -> Result<Option<run::Result<crate::Id>>> {
		// Attempt to get the evaluation result from the running evaluations.
		let state = self
			.state
			.running_evaluations
			.read()
			.unwrap()
			.get(&id)
			.cloned();
		if let Some(state) = state {
			let result = state.result().await;
			return Ok(Some(result));
		}

		// Attempt to get the evaluation children from the database or the parent.
		'a: {
			let Some(evaluation) = self.try_get_evaluation(id).await? else {
				break 'a;
			};
			return Ok(Some(evaluation.result));
		}

		Ok(None)
	}
}
