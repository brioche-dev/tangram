use crate::{
	build,
	document::{self, Document},
	evaluation, id, language, rid, value, Client, Error, Id, Result, Rid, WrapErr,
};
use async_recursion::async_recursion;
use futures::{stream::BoxStream, StreamExt, TryStreamExt};
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

	/// The pending evaluations.
	pub(crate) pending_evaluations:
		std::sync::RwLock<HashMap<Rid, Arc<evaluation::State>, rid::BuildHasher>>,

	/// The pending values_evaluations.
	pub(crate) pending_values_evaluations: std::sync::RwLock<HashMap<Id, Rid, id::BuildHasher>>,

	/// An HTTP client for downloading resources.
	pub(crate) resource_http_client: reqwest::Client,

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
	pub(crate) values_evaluations: lmdb::Database,
}

#[derive(Clone, Debug, Default)]
pub struct Options {
	pub parent_token: Option<String>,
	pub parent_url: Option<Url>,
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
		let values_evaluations = env.open_db(Some("values_evaluations"))?;
		let database = Database {
			env,
			values,
			evaluations,
			values_evaluations,
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

		// Create the pending evaluations.
		let pending_evaluations = std::sync::RwLock::new(HashMap::default());

		// Create the pending values to evaluations map.
		let pending_values_evaluations = std::sync::RwLock::new(HashMap::default());

		// Create the HTTP client for downloading resources.
		let resource_http_client = reqwest::Client::new();

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
			pending_evaluations,
			pending_values_evaluations,
			resource_http_client,
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

	#[async_recursion]
	pub async fn get_value_exists(&self, id: Id) -> Result<bool> {
		// Check if the value exists in the database.
		{
			let txn = self.state.database.env.begin_ro_txn()?;
			match txn.get(self.state.database.values, &id.as_bytes()) {
				Ok(_) => return Ok(true),
				Err(lmdb::Error::NotFound) => {},
				Err(error) => return Err(error.into()),
			};
		}

		// Check if the value exists in the parent.
		{
			if let Some(parent) = self.state.parent.as_ref() {
				if parent.get_value_exists(id).await? {
					return Ok(true);
				}
			}
		}

		Ok(false)
	}

	#[async_recursion]
	pub async fn try_get_value_bytes(&self, id: Id) -> Result<Option<Vec<u8>>> {
		// Attempt to get the value from the database.
		'a: {
			let txn = self.state.database.env.begin_ro_txn()?;
			let data = match txn.get(self.state.database.values, &id.as_bytes()) {
				Ok(data) => data,
				Err(lmdb::Error::NotFound) => break 'a,
				Err(error) => return Err(error.into()),
			};
			return Ok(Some(data.to_owned()));
		}

		// Attempt to get the value from the parent.
		'a: {
			let Some(parent) = self.state.parent.as_ref() else {
				break 'a;
			};

			// Get the value from the parent.
			let Some(bytes) = parent.try_get_value_bytes(id).await? else {
				break 'a;
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

	pub async fn try_get_evaluation_for_build(&self, build_id: build::Id) -> Result<Option<Rid>> {
		// Attempt to get the evaluation from the pending evaluations.
		if let Some(evaluation_id) = self
			.state
			.pending_values_evaluations
			.read()
			.unwrap()
			.get(&build_id)
		{
			return Ok(Some(*evaluation_id));
		}

		// Attempt to get the evaluation from the database.
		'a: {
			let txn = self.state.database.env.begin_ro_txn()?;
			let evaluation_id =
				match txn.get(self.state.database.values_evaluations, &build_id.as_bytes()) {
					Ok(evaluation_id) => evaluation_id,
					Err(lmdb::Error::NotFound) => break 'a,
					Err(error) => return Err(error.into()),
				};
			let evaluation_id = Rid::with_bytes(
				evaluation_id
					.try_into()
					.map_err(Error::other)
					.wrap_err("Invalid ID.")?,
			);
			return Ok(Some(evaluation_id));
		}

		// Attempt to get the evaluation from the parent.
		if let Some(parent) = self.state.parent.as_ref() {
			if let Some(evaluation_id) = parent.try_get_evaluation_for_build(build_id).await? {
				return Ok(Some(evaluation_id));
			}
		}

		Ok(None)
	}

	pub fn try_get_evaluation_children(
		&self,
		evaluation_id: Rid,
	) -> Result<Option<BoxStream<'static, Rid>>> {
		// Attempt to get the evaluation children from the pending evaluations.
		let state = self
			.state
			.pending_evaluations
			.read()
			.unwrap()
			.get(&evaluation_id)
			.cloned();
		if let Some(state) = state {
			return Ok(Some(state.children()));
		}

		// TODO: Attempt to get the evaluation children from the database.

		// TODO: Attempt to get the evaluation children from the parent.

		Ok(None)
	}

	pub async fn try_get_evaluation_log(
		&self,
		evaluations_id: Rid,
	) -> Result<Option<BoxStream<'static, Vec<u8>>>> {
		// Attempt to get the evaluation log from the pending evaluations.
		let state = self
			.state
			.pending_evaluations
			.read()
			.unwrap()
			.get(&evaluations_id)
			.cloned();
		if let Some(state) = state {
			let log = state.log().await?;
			return Ok(Some(log));
		}

		// TODO: Attempt to get the evaluation log from the database.

		// TODO: Attempt to get the evaluation log from the parent.

		Ok(None)
	}

	pub async fn try_get_evaluation_result(
		&self,
		evaluation_id: Rid,
	) -> Result<Option<evaluation::Result<Id>>> {
		// Attempt to get the evaluation result from the pending evaluations.
		let state = self
			.state
			.pending_evaluations
			.read()
			.unwrap()
			.get(&evaluation_id)
			.cloned();
		if let Some(state) = state {
			let result = state.result().await;
			return Ok(Some(result));
		}

		// TODO: Attempt to get the evaluation result from the database.

		// TODO: Attempt to get the evaluation result from the parent.

		Ok(None)
	}

	pub async fn evaluate(&self, build_id: build::Id) -> Result<Rid> {
		// Attempt to get an existing evaluation.
		if let Some(evaluation_id) = self.try_get_evaluation_for_build(build_id).await? {
			return Ok(evaluation_id);
		}

		// Create a new evaluation.
		let evaluation_id = Rid::gen();
		self.state
			.pending_evaluations
			.write()
			.unwrap()
			.insert(evaluation_id, todo!());
		self.state
			.pending_values_evaluations
			.write()
			.unwrap()
			.insert(*build_id, evaluation_id);
		Ok(evaluation_id)
	}
}

// pub async fn get_package_diagnostics(
// 	&self,
// 	package_id: package::Id,
// ) -> Result<Vec<language::Diagnostic>> {
// 	todo!()
// }

// pub async fn get_package_doc(&self, package_id: package::Id) -> Result<()> {
// 	todo!()
// }

// async fn store_direct(&self, server: &Server) -> Result<()> {
// 	// If the handle is already stored, then return.
// 	if self.id.read().unwrap().is_some() {
// 		return Ok(());
// 	}

// 	let handle = self.clone();
// 	let server = server.clone();
// 	tokio::task::spawn_blocking(move || {
// 		// Begin a write transaction.
// 		let mut txn = server.state.database.env.begin_rw_txn()?;

// 		// Collect the stored handles.
// 		let mut stored = Vec::new();

// 		// Store the handle and its unstored children recursively.
// 		handle.store_direct_inner(&server, &mut txn, &mut stored)?;

// 		// Commit the transaction.
// 		txn.commit()?;

// 		// Set the IDs of the stored handles.
// 		for (id, handle) in stored {
// 			handle.id.write().unwrap().replace(id);
// 		}

// 		Ok::<_, Error>(())
// 	})
// 	.await
// 	.map_err(Error::other)
// 	.wrap_err("Failed to join the store task.")?
// 	.wrap_err("Failed to store the value.")?;
// 	Ok(())
// }

// fn store_direct_inner(
// 	&self,
// 	server: &Server,
// 	txn: &mut lmdb::RwTransaction,
// 	stored: &mut Vec<(Id, Handle)>,
// ) -> Result<()> {
// 	// If the handle is already stored, then return.
// 	if self.id.read().unwrap().is_some() {
// 		return Ok(());
// 	}

// 	// Otherwise, it must be loaded, so get the value.
// 	let value = self.value.read().unwrap();
// 	let value = value.as_ref().unwrap();

// 	// Store the children.
// 	for child in value.children() {
// 		child.store_direct_inner(server, txn, stored)?;
// 	}

// 	// Serialize the data.
// 	let data = value.to_data();
// 	let bytes = data.serialize()?;
// 	let id = Id::new(self.kind(), &bytes);

// 	// Add the value to the database.
// 	txn.put(
// 		server.state.database.values,
// 		&id.as_bytes(),
// 		&bytes,
// 		lmdb::WriteFlags::empty(),
// 	)?;

// 	// Add to the stored handles.
// 	stored.push((id, self.clone()));

// 	Ok(())
// }

// async fn run_inner(&self, tg: &Server, parent: Option<Operation>) -> Result<Value> {
// 	// If the operation has already run, then return its output.
// 	let output = self.try_get_output(tg).await?;
// 	if let Some(output) = output {
// 		return Ok(output);
// 	}

// 	// Evaluate the operation.
// 	let output = match self {
// 		Build::Resource(resource) => resource.download_inner(tg).await?,
// 		Build::Target(target) => target.build_inner(tg).await?,
// 		Build::Task(task) => task.run_inner(tg).await?,
// 	};

// 	// Store the output.
// 	output.store(tg).await?;

// 	// Set the output.
// 	self.set_output_local(tg, &output).await?;

// 	Ok(output)
// }
