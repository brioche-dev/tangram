use self::progress::Progress;
use async_trait::async_trait;
use bytes::Bytes;
use futures::{future, stream::BoxStream, FutureExt};
use http_body_util::BodyExt;
use itertools::Itertools;
use lmdb::{Cursor, Transaction};
use std::{
	collections::HashMap,
	convert::Infallible,
	ffi::OsStr,
	net::SocketAddr,
	os::unix::prelude::OsStrExt,
	path::{Path, PathBuf},
	sync::{Arc, Weak},
};
use tangram_client as tg;
use tg::{Result, WrapErr};

mod build;
mod clean;
// mod fsm;
mod migrations;
mod object;
mod progress;

/// A server.
#[derive(Clone, Debug)]
pub struct Server {
	state: Arc<State>,
}

/// A server handle.
#[derive(Clone, Debug)]
pub struct Handle {
	state: Weak<State>,
}

/// Server state.
#[derive(Debug)]
struct State {
	/// The database.
	database: Database,

	/// A semaphore that prevents opening too many file descriptors.
	file_descriptor_semaphore: tokio::sync::Semaphore,

	/// The file system monitor task.
	// fsm_task: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,

	/// A local pool for running JS builds.
	local_pool: tokio_util::task::LocalPoolHandle,

	/// A client for communicating with the parent.
	parent: Option<Box<dyn tg::Client>>,

	/// The path to the directory where the server stores its data.
	path: PathBuf,

	/// The state of the server's running builds.
	running: std::sync::RwLock<(BuildForTargetMap, BuildProgressMap)>,
	// /// The VFS task.
	// vfs_task: std::sync::Mutex<Option<tokio::task::JoinHandle<Result<()>>>>,
}

type BuildForTargetMap = HashMap<tg::target::Id, tg::build::Id, fnv::FnvBuildHasher>;

type BuildProgressMap = HashMap<tg::build::Id, Progress, fnv::FnvBuildHasher>;

#[derive(Debug)]
struct Database {
	env: lmdb::Environment,
	objects: lmdb::Database,
	assignments: lmdb::Database,
	_trackers: lmdb::Database,
}

impl Server {
	pub async fn new(path: PathBuf, parent: Option<Box<dyn tg::Client>>) -> Result<Server> {
		// Ensure the path exists.
		tokio::fs::create_dir_all(&path)
			.await
			.wrap_err("Failed to create the directory.")?;

		// Migrate the path.
		Self::migrate(&path).await?;

		// Create the database.
		let database_path = path.join("database");
		let mut env_builder = lmdb::Environment::new();
		env_builder.set_map_size(1_099_511_627_776);
		env_builder.set_max_dbs(3);
		env_builder.set_max_readers(1024);
		env_builder.set_flags(lmdb::EnvironmentFlags::NO_SUB_DIR);
		let env = env_builder
			.open(&database_path)
			.wrap_err("Failed to open an environment.")?;
		let objects = env
			.open_db(Some("objects"))
			.wrap_err("Failed to open the objects database.")?;
		let assignments = env
			.open_db(Some("assignments"))
			.wrap_err("Failed to open the assignments database.")?;
		let trackers = env
			.open_db(Some("trackers"))
			.wrap_err("Failed to open the trackers datatabse.")?;

		delete_directory_trackers(&env, trackers)?;

		let database = Database {
			env,
			objects,
			assignments,
			_trackers: trackers,
		};

		// Create the file system semaphore.
		let file_descriptor_semaphore = tokio::sync::Semaphore::new(16);

		// Create the FSM task.
		// let fsm_task = tokio::sync::Mutex::new(None);

		// Create the local pool for running JS builds.
		let local_pool = tokio_util::task::LocalPoolHandle::new(
			std::thread::available_parallelism().unwrap().get(),
		);

		// Create the state of the server's running builds.
		let running = std::sync::RwLock::new((HashMap::default(), HashMap::default()));

		// Create the VFS task.
		// let vfs_task = std::sync::Mutex::new(None);

		// Create the state.
		let state = Arc::new(State {
			database,
			file_descriptor_semaphore,
			local_pool,
			parent,
			path,
			running,
			// vfs_task,
		});

		// Create the server.
		let server = Server { state };

		// // Start the FSM server.
		// let fsm = Fsm::new(Arc::downgrade(&server.state))?;
		// server.state.fsm.write().await.replace(fsm);

		// // Start the VFS server.
		// let vfs = vfs::Server::new(&server);
		// let task = vfs
		// 	.mount(server.artifacts_path())
		// 	.await
		// 	.wrap_err("Failed to mount the VFS.")?;
		// server.state.vfs_task.lock().unwrap().replace(task);

		Ok(server)
	}

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
	pub fn file_descriptor_semaphore(&self) -> &tokio::sync::Semaphore {
		&self.state.file_descriptor_semaphore
	}

	pub async fn serve(self, addr: SocketAddr) -> Result<()> {
		let listener = tokio::net::TcpListener::bind(&addr)
			.await
			.wrap_err("Failed to create a new tcp listener.")?;
		tracing::info!("🚀 Serving on {}.", addr);
		loop {
			let (stream, _) = listener
				.accept()
				.await
				.wrap_err("Failed to accept new incoming connections.")?;
			let stream = hyper_util::rt::TokioIo::new(stream);
			let server = self.clone();
			tokio::spawn(async move {
				hyper::server::conn::http2::Builder::new(hyper_util::rt::TokioExecutor::new())
					.serve_connection(
						stream,
						hyper::service::service_fn(move |request| {
							let server = server.clone();
							async move {
								tracing::info!(method = ?request.method(), path = ?request.uri().path(), "Received request.");
								let response = server.handle_request(request).await;
								tracing::info!(status = ?response.status(), "Sending response.");
								Ok::<_, Infallible>(response)
							}
						}),
					)
					.await
					.ok()
			});
		}
	}

	async fn handle_request(&self, request: http::Request<Incoming>) -> http::Response<Outgoing> {
		let method = request.method().clone();
		let path_components = request.uri().path().split('/').skip(1).collect_vec();
		let response = match (method, path_components.as_slice()) {
			// Objects
			(http::Method::HEAD, ["v1", "objects", _]) => {
				self.handle_head_object_request(request).map(Some).boxed()
			},
			(http::Method::GET, ["v1", "objects", _]) => {
				self.handle_get_object_request(request).map(Some).boxed()
			},
			(http::Method::PUT, ["v1", "objects", _]) => {
				self.handle_put_object_request(request).map(Some).boxed()
			},

			// Builds
			(http::Method::GET, ["v1", "targets", _, "build"]) => self
				.handle_try_get_build_for_target_request(request)
				.map(Some)
				.boxed(),
			(http::Method::POST, ["v1", "targets", _, "build"]) => self
				.handle_get_or_create_build_for_target_request(request)
				.map(Some)
				.boxed(),
			(http::Method::GET, ["v1", "builds", _, "children"]) => self
				.handle_try_get_build_children_request(request)
				.map(Some)
				.boxed(),
			(http::Method::GET, ["v1", "builds", _, "log"]) => {
				self.handle_get_build_log_request(request).map(Some).boxed()
			},
			(http::Method::GET, ["v1", "builds", _, "result"]) => self
				.handle_try_get_build_result_request(request)
				.map(Some)
				.boxed(),
			// (http::Method::GET, ["v1", _, "path"]) => self
			// 	.handle_get_object_for_path_request(request)
			// 	.map(Some)
			// 	.boxed(),
			// (http::Method::PUT, ["v1", _, "path"]) => self
			// 	.handle_put_object_for_path_request(request)
			// 	.map(Some)
			// 	.boxed(),
			(_, _) => future::ready(None).boxed(),
		}
		.await;
		match response {
			None => http::Response::builder()
				.status(http::StatusCode::NOT_FOUND)
				.body(full("Not found."))
				.unwrap(),
			Some(Err(error)) => {
				tracing::error!(?error);
				http::Response::builder()
					.status(http::StatusCode::INTERNAL_SERVER_ERROR)
					.body(full("Internal server error."))
					.unwrap()
			},
			Some(Ok(response)) => response,
		}
	}
}

impl tg::Handle for Handle {
	fn upgrade(&self) -> Option<Box<dyn tg::Client>> {
		self.state
			.upgrade()
			.map(|state| Box::new(Server { state }) as Box<dyn tg::Client>)
	}
}

#[async_trait]
impl tg::Client for Server {
	fn clone_box(&self) -> Box<dyn tg::Client> {
		Box::new(self.clone())
	}

	fn downgrade_box(&self) -> Box<dyn tg::Handle> {
		Box::new(Handle {
			state: Arc::downgrade(&self.state),
		})
	}

	fn path(&self) -> Option<&Path> {
		Some(self.path())
	}

	fn set_token(&self, _token: Option<String>) {}

	fn file_descriptor_semaphore(&self) -> &tokio::sync::Semaphore {
		&self.state.file_descriptor_semaphore
	}

	async fn get_object_exists(&self, id: &tg::object::Id) -> Result<bool> {
		self.get_object_exists(id).await
	}

	async fn get_object_bytes(&self, id: &tg::object::Id) -> Result<Vec<u8>> {
		self.get_object_bytes(id).await
	}

	async fn try_get_object_bytes(&self, id: &tg::object::Id) -> Result<Option<Vec<u8>>> {
		self.try_get_object_bytes(id).await
	}

	async fn try_put_object_bytes(
		&self,
		id: &tg::object::Id,
		bytes: &[u8],
	) -> Result<Result<(), Vec<tg::object::Id>>> {
		self.try_put_object_bytes(id, bytes).await
	}

	async fn try_get_build_for_target(&self, id: &tg::target::Id) -> Result<Option<tg::build::Id>> {
		self.try_get_build_for_target(id).await
	}

	async fn get_or_create_build_for_target(&self, id: &tg::target::Id) -> Result<tg::build::Id> {
		self.get_or_create_build_for_target(id).await
	}

	async fn try_get_build_children(
		&self,
		id: &tg::build::Id,
	) -> Result<Option<BoxStream<'static, Result<tg::build::Id>>>> {
		self.try_get_build_children(id).await
	}

	async fn try_get_build_log(
		&self,
		id: &tg::build::Id,
	) -> Result<Option<BoxStream<'static, Result<Bytes>>>> {
		self.try_get_build_log(id).await
	}

	async fn try_get_build_result(&self, id: &tg::build::Id) -> Result<Option<Result<tg::Value>>> {
		self.try_get_build_result(id).await
	}

	async fn clean(&self) -> Result<()> {
		self.clean().await
	}

	async fn create_login(&self) -> Result<tg::user::Login> {
		self.state
			.parent
			.as_ref()
			.wrap_err("The server does not have a parent.")?
			.create_login()
			.await
	}

	async fn get_login(&self, id: &tg::Id) -> Result<Option<tg::user::Login>> {
		self.state
			.parent
			.as_ref()
			.wrap_err("The server does not have a parent.")?
			.get_login(id)
			.await
	}

	async fn publish_package(&self, id: &tg::package::Id) -> Result<()> {
		self.state
			.parent
			.as_ref()
			.wrap_err("The server does not have a parent.")?
			.publish_package(id)
			.await
	}

	async fn search_packages(&self, query: &str) -> Result<Vec<tg::package::SearchResult>> {
		self.state
			.parent
			.as_ref()
			.wrap_err("The server does not have a parent.")?
			.search_packages(query)
			.await
	}

	async fn get_current_user(&self) -> Result<tg::user::User> {
		self.state
			.parent
			.as_ref()
			.wrap_err("The server does not have a parent.")?
			.get_current_user()
			.await
	}

	async fn try_get_artifact_for_path(&self, path: &Path) -> Result<Option<tg::Artifact>> {
		self.try_get_artifact_for_path(path).await
	}

	async fn try_get_package_for_path(&self, path: &Path) -> Result<Option<tg::Package>> {
		self.try_get_package_for_path(path).await
	}

	async fn set_artifact_for_path(&self, path: &Path, artifact: &tg::Artifact) -> Result<()> {
		self.set_artifact_for_path(path, artifact).await
	}

	async fn set_package_for_path(&self, path: &Path, package: &tg::Package) -> Result<()> {
		self.set_package_for_path(path, package).await
	}
}

pub type Incoming = hyper::body::Incoming;
pub type Outgoing = http_body_util::combinators::UnsyncBoxBody<
	::bytes::Bytes,
	Box<dyn std::error::Error + Send + Sync + 'static>,
>;

/// An empty response body.
#[must_use]
pub fn empty() -> Outgoing {
	http_body_util::Empty::new()
		.map_err(|_| unreachable!())
		.boxed_unsync()
}

/// A full response body.
#[must_use]
pub fn full(chunk: impl Into<::bytes::Bytes>) -> Outgoing {
	http_body_util::Full::new(chunk.into())
		.map_err(|_| unreachable!())
		.boxed_unsync()
}

/// 200
#[must_use]
pub fn ok() -> http::Response<Outgoing> {
	http::Response::builder()
		.status(http::StatusCode::OK)
		.body(empty())
		.unwrap()
}

/// 400
#[must_use]
pub fn bad_request() -> http::Response<Outgoing> {
	http::Response::builder()
		.status(http::StatusCode::BAD_REQUEST)
		.body(full("Bad request."))
		.unwrap()
}

/// 404
#[must_use]
pub fn not_found() -> http::Response<Outgoing> {
	http::Response::builder()
		.status(http::StatusCode::NOT_FOUND)
		.body(full("Not found."))
		.unwrap()
}

fn delete_directory_trackers(env: &lmdb::Environment, trackers: lmdb::Database) -> Result<()> {
	let paths = {
		let txn = env
			.begin_ro_txn()
			.wrap_err("Failed to begin the transaction.")?;
		let mut cursor = txn
			.open_ro_cursor(trackers)
			.wrap_err("Failed to open the cursor.")?;
		cursor
			.iter()
			.filter_map(|entry| {
				let (path, _) = entry.ok()?;
				let path = PathBuf::from(OsStr::from_bytes(path));
				path.is_dir().then_some(path)
			})
			.collect::<Vec<_>>()
	};

	let mut txn = env
		.begin_rw_txn()
		.wrap_err("Failed to begin the transaction.")?;
	for path in paths {
		let key = path.as_os_str().as_bytes();
		let _ = txn.del(trackers, &key, None);
	}
	txn.commit().wrap_err("Failed to commit the transaction.")?;
	Ok(())
}
