use crate::{
	document::{self, Document},
	id, language, object, run, system, task, vfs, Blob, Client, Error, Result, Run, System, Task,
	Value, WrapErr,
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
use tokio_util::io::StreamReader;
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

	/// An HTTP client for downloading resources.
	pub(crate) http_client: reqwest::Client,

	/// A channel sender to send requests to the language service.
	pub(crate) language_service_request_sender:
		std::sync::Mutex<Option<language::service::RequestSender>>,

	/// A local pool for running JS tasks.
	pub(crate) local_pool: tokio_util::task::LocalPoolHandle,

	/// A handle to the main tokio runtime.
	pub(crate) main_runtime_handle: tokio::runtime::Handle,

	/// The options the server was created with.
	pub(crate) options: Options,

	/// A client for communicating with the parent.
	pub(crate) parent: Option<Client>,

	/// The path to the directory where the server stores its data.
	pub(crate) path: PathBuf,

	/// A semaphore that limits the number of concurrent subprocesses.
	pub(crate) process_semaphore: tokio::sync::Semaphore,

	/// The server's uncompleted runs.
	pub(crate) uncompleted_runs:
		std::sync::RwLock<HashMap<run::Id, Arc<run::State>, id::BuildHasher>>,

	pub(crate) vfs_server_task: std::sync::Mutex<Option<tokio::task::JoinHandle<Result<()>>>>,
}

#[derive(Debug)]
pub struct Database {
	pub(crate) env: lmdb::Environment,
	pub(crate) objects: lmdb::Database,
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
		let objects = env.open_db(Some("objects"))?;
		let assignments = env.open_db(Some("assignments"))?;
		let database = Database {
			env,
			objects,
			assignments,
		};

		// Create the documents maps.
		let documents = tokio::sync::RwLock::new(HashMap::default());

		// Create the file system semaphore.
		let file_descriptor_semaphore = tokio::sync::Semaphore::new(16);

		// Create the HTTP client for downloading resources.
		let http_client = reqwest::Client::new();

		// Create the sender for language service requests.
		let language_service_request_sender = std::sync::Mutex::new(None);

		// Create the local pool for running JS tasks.
		let local_pool = tokio_util::task::LocalPoolHandle::new(
			std::thread::available_parallelism().unwrap().get(),
		);

		// Get the curent tokio runtime handler.
		let main_runtime_handle = tokio::runtime::Handle::current();

		// Create the parent client.
		let parent = if let Some(url) = options.parent_url.as_ref() {
			let token = options.parent_token.clone();
			Some(Client::with_url(url.clone(), token))
		} else {
			None
		};

		// Create the process semaphore.
		let process_semaphore =
			tokio::sync::Semaphore::new(std::thread::available_parallelism().unwrap().get());

		// Create the runs.
		let uncompleted_runs = std::sync::RwLock::new(HashMap::default());

		// Create the VFS server task.
		let vfs_server_task = std::sync::Mutex::new(None);

		// Create the state.
		let state = State {
			database,
			documents,
			file_descriptor_semaphore,
			http_client,
			language_service_request_sender,
			local_pool,
			main_runtime_handle,
			options,
			parent,
			path,
			process_semaphore,
			uncompleted_runs,
			vfs_server_task,
		};

		// Create the server.
		let server = Server {
			state: Arc::new(state),
		};

		// Start the VFS server.
		server
			.state
			.vfs_server_task
			.lock()
			.unwrap()
			.replace(tokio::spawn({
				let path = server.artifacts_path();
				let client = Client::with_server(server.clone());
				async move {
					let vfs_server = vfs::Server::new(path, client);
					vfs_server.serve().await
				}
			}));

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

	pub(crate) async fn get_object_exists(&self, id: object::Id) -> Result<bool> {
		// Check if the object exists in the database.
		if self.get_object_exists_from_database(id)? {
			return Ok(true);
		}

		// Check if the object exists in the parent.
		if self.get_object_exists_from_parent(id).await? {
			return Ok(true);
		}

		Ok(false)
	}

	pub(crate) fn get_object_exists_from_database(&self, id: object::Id) -> Result<bool> {
		let txn = self.state.database.env.begin_ro_txn()?;
		match txn.get(self.state.database.objects, &id.as_bytes()) {
			Ok(_) => Ok(true),
			Err(lmdb::Error::NotFound) => Ok(false),
			Err(error) => Err(error.into()),
		}
	}

	async fn get_object_exists_from_parent(&self, id: object::Id) -> Result<bool> {
		if let Some(parent) = self.state.parent.as_ref() {
			if parent.get_object_exists(id).await? {
				return Ok(true);
			}
		}
		Ok(false)
	}

	pub(crate) async fn get_object_bytes(&self, id: object::Id) -> Result<Vec<u8>> {
		self.try_get_object_bytes(id)
			.await?
			.wrap_err("Failed to get the object.")
	}

	pub(crate) async fn try_get_object_bytes(&self, id: object::Id) -> Result<Option<Vec<u8>>> {
		// Attempt to get the object from the database.
		if let Some(bytes) = self.try_get_object_bytes_from_database(id)? {
			return Ok(Some(bytes));
		};

		// Attempt to get the object from the parent.
		if let Some(bytes) = self.try_get_object_bytes_from_parent(id).await? {
			return Ok(Some(bytes));
		};

		Ok(None)
	}

	pub fn try_get_object_bytes_from_database(&self, id: object::Id) -> Result<Option<Vec<u8>>> {
		let txn = self.state.database.env.begin_ro_txn()?;
		match txn.get(self.state.database.objects, &id.as_bytes()) {
			Ok(data) => Ok(Some(data.to_owned())),
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => Err(error.into()),
		}
	}

	async fn try_get_object_bytes_from_parent(&self, id: object::Id) -> Result<Option<Vec<u8>>> {
		let Some(parent) = self.state.parent.as_ref() else {
			return Ok(None);
		};

		// Get the object from the parent.
		let Some(bytes) = parent.try_get_object_bytes(id).await? else {
			return Ok(None);
		};

		// Create a write transaction.
		let mut txn = self.state.database.env.begin_rw_txn()?;

		// Add the object to the database.
		txn.put(
			self.state.database.objects,
			&id.as_bytes(),
			&bytes,
			lmdb::WriteFlags::empty(),
		)?;

		// Commit the transaction.
		txn.commit()?;

		Ok(Some(bytes))
	}

	/// Attempt to put a object.
	pub(crate) async fn try_put_object_bytes(
		&self,
		id: object::Id,
		bytes: &[u8],
	) -> Result<Result<(), Vec<object::Id>>> {
		// Deserialize the data.
		let data = object::Data::deserialize(id.kind(), bytes)?;

		// Check if there are any missing children.
		let missing_children = stream::iter(data.children())
			.map(Ok)
			.try_filter_map(|id| async move {
				let exists = self.get_object_exists(id).await?;
				Ok::<_, Error>(if exists { None } else { Some(id) })
			})
			.try_collect::<Vec<_>>()
			.await?;
		if !missing_children.is_empty() {
			return Ok(Err(missing_children));
		}

		// Create a write transaction.
		let mut txn = self.state.database.env.begin_rw_txn()?;

		// Add the object to the database.
		txn.put(
			self.state.database.objects,
			&id.as_bytes(),
			&bytes,
			lmdb::WriteFlags::empty(),
		)?;

		// Commit the transaction.
		txn.commit()?;

		Ok(Ok(()))
	}

	/// Attempt to get the run for a task.
	pub(crate) async fn try_get_run_for_task(&self, id: task::Id) -> Result<Option<run::Id>> {
		// Attempt to get the run for the task from the database.
		if let Some(run_id) = self.try_get_run_for_task_from_database(id)? {
			return Ok(Some(run_id));
		}

		// Attempt to get the run for the task from the parent.
		if let Some(run_id) = self.try_get_run_for_task_from_parent(id).await? {
			return Ok(Some(run_id));
		}

		Ok(None)
	}

	/// Attempt to get the run for the task from the database.
	fn try_get_run_for_task_from_database(&self, id: task::Id) -> Result<Option<run::Id>> {
		// Get the run for the task from the database.
		let txn = self.state.database.env.begin_ro_txn()?;
		match txn.get(self.state.database.assignments, &id.as_bytes()) {
			Ok(run_id) => Ok(Some(run_id.try_into().wrap_err("Invalid ID.")?)),
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => Err(error.into()),
		}
	}

	/// Attempt to get the run for the task from the parent.
	async fn try_get_run_for_task_from_parent(&self, id: task::Id) -> Result<Option<run::Id>> {
		// Get the parent.
		let Some(parent) = self.state.parent.as_ref() else {
			return Ok(None);
		};

		// Get the assignment.
		let Some(run_id) = parent.try_get_run_for_task(id).await? else {
			return Ok(None);
		};

		// Create a write transaction.
		let mut txn = self.state.database.env.begin_rw_txn()?;

		// Set the run for the task in the database.
		txn.put(
			self.state.database.assignments,
			&id.as_bytes(),
			&run_id.as_bytes(),
			lmdb::WriteFlags::empty(),
		)?;

		// Commit the transaction.
		txn.commit()?;

		Ok(Some(run_id))
	}

	/// Get or create a run for a task.
	pub(crate) async fn get_or_create_run_for_task(&self, task_id: task::Id) -> Result<run::Id> {
		// Attempt to get the run for the task.
		if let Some(run_id) = self.try_get_run_for_task(task_id).await? {
			return Ok(run_id);
		}

		// Otherwise, create a new run.
		let run_id = run::Id::new();
		let state = Arc::new(run::State::new()?);
		self.state
			.uncompleted_runs
			.write()
			.unwrap()
			.insert(run_id, state.clone());

		// Create a write transaction.
		let mut txn = self.state.database.env.begin_rw_txn()?;

		// Set the run for the task in the database.
		txn.put(
			self.state.database.assignments,
			&task_id.as_bytes(),
			&run_id.as_bytes(),
			lmdb::WriteFlags::empty(),
		)?;

		// Commit the transaction.
		txn.commit()?;

		// Spawn the task.
		tokio::spawn({
			let task = Task::with_id(task_id);
			let server = self.clone();
			async move {
				let client = &Client::with_server(server.clone());

				let object = task.object(client).await?;

				let result = match object.host.os() {
					_ => Ok(Value::String("Hello, World".to_owned())),
				};

				// Set the result on the state.
				state.set_result(result).await;

				// Create the object.
				let children = state.children().try_collect().await?;
				let log = StreamReader::new(
					state
						.log()
						.await?
						.map_ok(::bytes::Bytes::from)
						.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error)),
				);
				let log = Blob::with_reader(client, log).await?;
				let result = state.result().await;
				let object = run::Object {
					children,
					log,
					result,
				};

				// Store the children.
				object
					.children()
					.into_iter()
					.map(|child| async move { child.store(client).await })
					.collect::<futures::stream::FuturesUnordered<_>>()
					.try_collect()
					.await?;

				// Get the data.
				let data = object.to_data();
				let bytes = data.serialize()?;

				// Store the object.
				client
					.try_put_object_bytes(run_id.into(), &bytes)
					.await
					.wrap_err("Failed to put the object.")?
					.ok()
					.wrap_err("Expected all children to be stored.")?;

				// Remove the run from the uncompleted runs.
				server
					.state
					.uncompleted_runs
					.write()
					.unwrap()
					.remove(&run_id);

				Ok::<_, Error>(())
			}
		});

		Ok(run_id)
	}

	pub(crate) async fn try_get_run_children(
		&self,
		id: run::Id,
	) -> Result<Option<BoxStream<'static, Result<run::Id>>>> {
		let client = &Client::with_server(self.clone());
		let run = Run::with_id(id);

		// Attempt to stream the children from the uncompleted runs.
		let state = self
			.state
			.uncompleted_runs
			.read()
			.unwrap()
			.get(&run.id())
			.cloned();
		if let Some(state) = state {
			let children = state.children();
			return Ok(Some(children.map_ok(|child| child.id()).boxed()));
		}

		// Attempt to get the children from the object.
		'a: {
			let Some(object) = run.try_get_object(client).await? else {
				break 'a;
			};
			return Ok(Some(
				stream::iter(object.children.clone())
					.map(|child| child.id())
					.map(Ok)
					.boxed(),
			));
		}

		// Attempt to stream the children from the parent.
		'a: {
			let Some(parent) = self.state.parent.as_ref() else {
				break 'a;
			};
			let Some(children) = parent.try_get_run_children(id).await? else {
				break 'a;
			};
			return Ok(Some(children));
		}

		Ok(None)
	}

	pub(crate) async fn try_get_run_log(
		&self,
		id: run::Id,
	) -> Result<Option<BoxStream<'static, Result<Vec<u8>>>>> {
		let client = &Client::with_server(self.clone());
		let run = Run::with_id(id);

		// Attempt to stream the log from the uncompleted runs.
		let state = self
			.state
			.uncompleted_runs
			.read()
			.unwrap()
			.get(&run.id())
			.cloned();
		if let Some(state) = state {
			let log = state.log().await?;
			return Ok(Some(log));
		}

		// Attempt to get the log from the object.
		'a: {
			let Some(object) = run.try_get_object(client).await? else {
				break 'a;
			};
			let object = object.clone();
			let client = client.clone();
			return Ok(Some(
				stream::once(async move { object.log.bytes(&client).await }).boxed(),
			));
		}

		// Attempt to stream the log from the parent.
		'a: {
			let Some(parent) = self.state.parent.as_ref() else {
				break 'a;
			};
			let Some(log) = parent.try_get_run_log(id).await? else {
				break 'a;
			};
			return Ok(Some(log));
		}

		Ok(None)
	}

	pub(crate) async fn try_get_run_result(&self, id: run::Id) -> Result<Option<Result<Value>>> {
		let client = &Client::with_server(self.clone());
		let run = Run::with_id(id);

		// Attempt to await the result from the uncompleted runs.
		let state = self
			.state
			.uncompleted_runs
			.read()
			.unwrap()
			.get(&run.id())
			.cloned();
		if let Some(state) = state {
			let result = state.result().await;
			return Ok(Some(result));
		}

		// Attempt to get the result from the object.
		'a: {
			let Some(object) = run.try_get_object(client).await? else {
				break 'a;
			};
			return Ok(Some(object.result.clone()));
		}

		// Attempt to await the result from the parent.
		'a: {
			let Some(parent) = self.state.parent.as_ref() else {
				break 'a;
			};
			let Some(result) = parent.try_get_run_result(id).await? else {
				break 'a;
			};
			return Ok(Some(result));
		}

		Ok(None)
	}
}

impl Drop for Server {
	fn drop(&mut self) {
		// Abort the VFS server task.
		if let Some(task) = self.state.vfs_server_task.lock().unwrap().take() {
			task.abort();
		}
	}
}
