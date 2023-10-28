use self::progress::Progress;
use async_trait::async_trait;
use bytes::Bytes;
use futures::{future, stream::BoxStream, FutureExt};
use hyper_util::rt::TokioIo;
use itertools::Itertools;
use std::{
	collections::HashMap,
	convert::Infallible,
	os::fd::AsRawFd,
	path::{Path, PathBuf},
	sync::{Arc, Weak},
};
use tangram_client as tg;
use tangram_util::{
	http::{full, ok, Incoming, Outgoing},
	net::Addr,
};
use tg::{util::rmrf, Result, Wrap, WrapErr};
use tokio::net::{TcpListener, UnixListener};
use tokio_util::either::Either;

mod build;
mod clean;
// mod fsm;
mod migrations;
mod object;
mod progress;

/// A server.
#[derive(Clone, Debug)]
pub struct Server {
	inner: Arc<Inner>,
}

/// A server handle.
#[derive(Clone, Debug)]
pub struct Handle {
	inner: Weak<Inner>,
}

#[derive(Debug)]
struct Inner {
	/// The server's running builds.
	builds: std::sync::RwLock<(BuildForTargetMap, BuildProgressMap)>,

	/// The database.
	database: Database,

	/// A semaphore that prevents opening too many file descriptors.
	file_descriptor_semaphore: tokio::sync::Semaphore,

	/// The file system monitor task.
	// fsm_task: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,

	/// A local pool for build JS targets.
	local_pool: tokio_util::task::LocalPoolHandle,

	/// The lock file.
	#[allow(dead_code)]
	lock_file: tokio::fs::File,

	/// A client for communicating with the parent.
	parent: Option<Box<dyn tg::Client>>,

	/// The path to the directory where the server stores its data.
	path: PathBuf,

	/// The VFS task.
	vfs_task: std::sync::Mutex<Option<tokio::task::JoinHandle<Result<()>>>>,
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

		// Acquire an exclusive lock to the path.
		let lock_file = tokio::fs::OpenOptions::new()
			.read(true)
			.write(true)
			.create(true)
			.open(path.join("lock"))
			.await
			.wrap_err("Failed to open the lock file.")?;
		let ret = unsafe { libc::flock(lock_file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
		if ret != 0 {
			return Err(std::io::Error::last_os_error().wrap("Failed to acquire the lock file."));
		}

		// Migrate the path.
		Self::migrate(&path).await?;

		// Remove an existing socket file.
		rmrf(&path.join("socket"))
			.await
			.wrap_err("Failed to remove an existing socket file.")?;

		// Create the server's running builds.
		let builds = std::sync::RwLock::new((HashMap::default(), HashMap::default()));

		// Open the database.
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

		// Create the local pool.
		let local_pool = tokio_util::task::LocalPoolHandle::new(
			std::thread::available_parallelism().unwrap().get(),
		);

		// Create the VFS task.
		let vfs_task = std::sync::Mutex::new(None);

		// Create the inner.
		let inner = Arc::new(Inner {
			builds,
			database,
			file_descriptor_semaphore,
			local_pool,
			lock_file,
			parent,
			path,
			vfs_task,
		});

		// Create the server.
		let server = Server { inner };

		// // Start the FSM server.
		// let fsm = Fsm::new(Arc::downgrade(&server.inner))?;
		// server.inner.fsm.write().await.replace(fsm);

		// Start the VFS server.
		let vfs = tangram_vfs::Server::new(&server);
		let task = vfs
			.mount(&server.artifacts_path())
			.await
			.wrap_err("Failed to mount the VFS.")?;
		server.inner.vfs_task.lock().unwrap().replace(task);

		Ok(server)
	}

	#[must_use]
	pub fn path(&self) -> &Path {
		&self.inner.path
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
		&self.inner.file_descriptor_semaphore
	}

	pub async fn serve(self, addr: Addr) -> Result<()> {
		let listener = match &addr {
			Addr::Inet(inet) => Either::Left(
				TcpListener::bind(inet.to_string())
					.await
					.wrap_err("Failed to create the TCP listener.")?,
			),
			Addr::Unix(path) => Either::Right(
				UnixListener::bind(path).wrap_err("Failed to create the UNIX listener.")?,
			),
		};
		tracing::info!("🚀 Serving on {addr:?}.");
		loop {
			let stream = TokioIo::new(match &listener {
				Either::Left(listener) => Either::Left(
					listener
						.accept()
						.await
						.wrap_err("Failed to accept a new TCP connection.")?
						.0,
				),
				Either::Right(listener) => Either::Right(
					listener
						.accept()
						.await
						.wrap_err("Failed to accept a new UNIX connection.")?
						.0,
				),
			});
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
			(http::Method::GET, ["v1", "ping"]) => future::ready(Some(Ok(ok()))).boxed(),
			(http::Method::POST, ["v1", "stop"]) => {
				self.handle_stop_request(request).map(Some).boxed()
			},

			// Clean
			(http::Method::POST, ["v1", "clean"]) => {
				self.handle_clean_request(request).map(Some).boxed()
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
			(http::Method::GET, ["v1", "builds", _, "target"]) => self
				.handle_try_get_build_target_request(request)
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

			// Package

			// Paths
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

	async fn handle_stop_request(
		&self,
		_request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		std::process::exit(0);
	}

	async fn ping(&self) -> Result<()> {
		Ok(())
	}

	async fn stop(&self) -> Result<()> {
		Ok(())
	}
}

impl tg::Handle for Handle {
	fn upgrade(&self) -> Option<Box<dyn tg::Client>> {
		self.inner
			.upgrade()
			.map(|inner| Box::new(Server { inner }) as Box<dyn tg::Client>)
	}
}

#[async_trait]
impl tg::Client for Server {
	fn clone_box(&self) -> Box<dyn tg::Client> {
		Box::new(self.clone())
	}

	fn downgrade_box(&self) -> Box<dyn tg::Handle> {
		Box::new(Handle {
			inner: Arc::downgrade(&self.inner),
		})
	}

	fn path(&self) -> Option<&Path> {
		Some(self.path())
	}

	fn set_token(&self, _token: Option<String>) {}

	fn file_descriptor_semaphore(&self) -> &tokio::sync::Semaphore {
		&self.inner.file_descriptor_semaphore
	}

	async fn ping(&self) -> Result<()> {
		self.ping().await
	}

	async fn stop(&self) -> Result<()> {
		self.stop().await
	}

	async fn clean(&self) -> Result<()> {
		self.clean().await
	}

	async fn get_object_exists(&self, id: &tg::object::Id) -> Result<bool> {
		self.get_object_exists(id).await
	}

	async fn get_object_bytes(&self, id: &tg::object::Id) -> Result<Bytes> {
		self.get_object_bytes(id).await
	}

	async fn try_get_object_bytes(&self, id: &tg::object::Id) -> Result<Option<Bytes>> {
		self.try_get_object_bytes(id).await
	}

	async fn try_put_object_bytes(
		&self,
		id: &tg::object::Id,
		bytes: &Bytes,
	) -> Result<Result<(), Vec<tg::object::Id>>> {
		self.try_put_object_bytes(id, bytes).await
	}

	async fn try_get_build_for_target(&self, id: &tg::target::Id) -> Result<Option<tg::build::Id>> {
		self.try_get_build_for_target(id).await
	}

	async fn get_or_create_build_for_target(&self, id: &tg::target::Id) -> Result<tg::build::Id> {
		self.get_or_create_build_for_target(id).await
	}

	async fn try_get_build_queue_item(&self) -> Result<Option<tg::build::Id>> {
		todo!()
	}

	async fn try_get_build_target(&self, id: &tg::build::Id) -> Result<Option<tg::target::Id>> {
		self.try_get_build_target(id).await
	}

	async fn try_finish_build(&self, id: &tg::build::Id) -> Result<()> {
		todo!()
	}

	async fn try_get_build_children(
		&self,
		id: &tg::build::Id,
	) -> Result<Option<BoxStream<'static, Result<tg::build::Id>>>> {
		self.try_get_build_children(id).await
	}

	async fn try_put_build_child(
		&self,
		build_id: &tg::build::Id,
		child_id: &tg::build::Id,
	) -> Result<()> {
		todo!()
	}

	async fn try_get_build_log(
		&self,
		id: &tg::build::Id,
	) -> Result<Option<BoxStream<'static, Result<Bytes>>>> {
		self.try_get_build_log(id).await
	}

	async fn try_put_build_log(&self, build_id: &tg::build::Id, bytes: Bytes) -> Result<()> {
		todo!()
	}

	async fn try_get_build_result(&self, id: &tg::build::Id) -> Result<Option<Result<tg::Value>>> {
		self.try_get_build_result(id).await
	}

	async fn try_put_build_result(
		&self,
		build_id: &tg::build::Id,
		result: tg::Value,
	) -> Result<()> {
		todo!()
	}

	async fn create_login(&self) -> Result<tg::user::Login> {
		self.inner
			.parent
			.as_ref()
			.wrap_err("The server does not have a parent.")?
			.create_login()
			.await
	}

	async fn get_login(&self, id: &tg::Id) -> Result<Option<tg::user::Login>> {
		self.inner
			.parent
			.as_ref()
			.wrap_err("The server does not have a parent.")?
			.get_login(id)
			.await
	}

	async fn publish_package(&self, id: &tg::package::Id) -> Result<()> {
		self.inner
			.parent
			.as_ref()
			.wrap_err("The server does not have a parent.")?
			.publish_package(id)
			.await
	}

	async fn search_packages(&self, query: &str) -> Result<Vec<tg::package::SearchResult>> {
		self.inner
			.parent
			.as_ref()
			.wrap_err("The server does not have a parent.")?
			.search_packages(query)
			.await
	}

	async fn get_current_user(&self) -> Result<tg::user::User> {
		self.inner
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
