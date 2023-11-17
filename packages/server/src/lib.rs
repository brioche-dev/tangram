use self::builder::Builder;
use async_trait::async_trait;
use bytes::Bytes;
use database::Database;
use futures::stream::BoxStream;
use std::{
	collections::HashMap,
	os::fd::AsRawFd,
	path::{Path, PathBuf},
	sync::Arc,
};
use tangram_client as tg;
use tangram_error::{Result, Wrap, WrapErr};
use tangram_http::net::Addr;
use tangram_lsp::package::Ext;
use tg::util::rmrf;

mod build;
mod builder;
mod clean;
mod database;
mod migrations;
mod object;
mod tracker;

/// A server.
#[derive(Clone)]
pub struct Server {
	inner: Arc<Inner>,
}

struct Inner {
	/// The state of the server's builds.
	builds: std::sync::RwLock<HashMap<tg::build::Id, BuildState, fnv::FnvBuildHasher>>,

	/// The builder.
	builder: std::sync::Mutex<Option<Builder>>,

	/// The database.
	database: Database,

	/// A semaphore that prevents opening too many file descriptors.
	file_descriptor_semaphore: tokio::sync::Semaphore,

	/// The fsm.
	// fsm: tokio::sync::Mutex<Option<Fsm>>,

	/// The HTTP server.
	http: std::sync::Mutex<Option<tangram_http::Server>>,

	/// A local pool for build JS targets.
	local_pool: tokio_util::task::LocalPoolHandle,

	/// The lock file.
	#[allow(dead_code)]
	lock_file: tokio::fs::File,

	/// The path to the directory where the server stores its data.
	path: PathBuf,

	/// A client for communicating with the parent server.
	remote: Option<Box<dyn tg::Client>>,

	/// The server's version.
	version: String,

	/// The VFS server.
	vfs: std::sync::Mutex<Option<tangram_vfs::Server>>,
}

#[derive(Clone, Debug)]
struct BuildState {
	inner: Arc<BuildStateInner>,
}

#[derive(Debug)]
struct BuildStateInner {
	target: tg::Target,
	children: std::sync::Mutex<ChildrenState>,
	log: Arc<tokio::sync::Mutex<LogState>>,
	result: ResultState,
}

#[derive(Debug)]
struct ChildrenState {
	children: Vec<tg::Build>,
	sender: Option<tokio::sync::broadcast::Sender<tg::Build>>,
}

#[derive(Debug)]
struct LogState {
	file: tokio::fs::File,
	sender: Option<tokio::sync::broadcast::Sender<Bytes>>,
}

#[derive(Debug)]
struct ResultState {
	result: tokio::sync::watch::Receiver<Option<Result<tg::Value>>>,
	sender: tokio::sync::watch::Sender<Option<Result<tg::Value>>>,
}

pub struct Options {
	pub addr: Addr,
	pub remote: Option<Box<dyn tg::Client>>,
	pub path: PathBuf,
	pub version: String,
}

impl Server {
	pub async fn start(options: Options) -> Result<Server> {
		let Options {
			addr,
			remote,
			path,
			version,
		} = options;

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

		// Write the PID file.
		tokio::fs::write(&path.join("server.pid"), std::process::id().to_string())
			.await
			.wrap_err("Failed to write the PID file.")?;

		// Remove an existing socket file.
		rmrf(&path.join("socket"))
			.await
			.wrap_err("Failed to remove an existing socket file.")?;

		// Create the state of the server's builds.
		let builds = std::sync::RwLock::new(HashMap::default());

		// Create the builder.
		let builder = std::sync::Mutex::new(None);

		// Open the database.
		let database = Database::open(&path.join("database"))?;

		// Create the file system semaphore.
		let file_descriptor_semaphore = tokio::sync::Semaphore::new(16);

		// Create the FSM.
		// let fsm_task = tokio::sync::Mutex::new(None);

		// Create the HTTP server.
		let http = std::sync::Mutex::new(None);

		// Create the local pool.
		let local_pool = tokio_util::task::LocalPoolHandle::new(
			std::thread::available_parallelism().unwrap().get(),
		);

		// Create the VFS.
		let vfs = std::sync::Mutex::new(None);

		// Create the inner.
		let inner = Arc::new(Inner {
			builds,
			builder,
			database,
			file_descriptor_semaphore,
			http,
			local_pool,
			lock_file,
			path,
			remote,
			version,
			vfs,
		});

		// Create the server.
		let server = Server { inner };

		// // Start the FSM server.
		// let fsm = Fsm::new(Arc::downgrade(&server.inner))?;
		// server.inner.fsm.write().await.replace(fsm);

		// Start the VFS server.
		let vfs = tangram_vfs::Server::start(&server, &server.artifacts_path())
			.await
			.wrap_err("Failed to start the VFS server.")?;
		server.inner.vfs.lock().unwrap().replace(vfs);

		// Start the HTTP server.
		let http = tangram_http::Server::start(&server, addr, None)
			.await
			.wrap_err("Failed to start the HTTP server.")?;
		server.inner.http.lock().unwrap().replace(http);

		// Start the builder.
		if server.inner.remote.is_some() {
			let builder = Builder::start(&server);
			server.inner.builder.lock().unwrap().replace(builder);
		}

		Ok(server)
	}

	pub fn stop(&self) {
		// Stop the HTTP server.
		if let Some(http) = self.inner.http.lock().unwrap().as_ref() {
			http.stop();
		}

		// Stop the builder.
		if let Some(builder) = self.inner.builder.lock().unwrap().as_ref() {
			builder.stop();
		}
	}

	pub async fn join(&self) -> Result<()> {
		// Join the builder.
		let builder = self.inner.builder.lock().unwrap().clone();
		if let Some(builder) = builder {
			builder.join().await?;
		}

		// Join the HTTP server.
		let http = self.inner.http.lock().unwrap().clone();
		if let Some(http) = http {
			http.join().await?;
		}

		// Join the VFS server.
		let vfs = self.inner.vfs.lock().unwrap().clone();
		if let Some(vfs) = vfs {
			vfs.stop();
			vfs.join().await?;
		}

		Ok(())
	}

	#[allow(clippy::unused_async)]
	async fn status(&self) -> Result<tg::status::Status> {
		Ok(tg::status::Status {
			version: self.inner.version.clone(),
		})
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
}

#[async_trait]
impl tg::Client for Server {
	fn clone_box(&self) -> Box<dyn tg::Client> {
		Box::new(self.clone())
	}

	fn path(&self) -> Option<&Path> {
		Some(self.path())
	}

	fn file_descriptor_semaphore(&self) -> &tokio::sync::Semaphore {
		&self.inner.file_descriptor_semaphore
	}

	async fn status(&self) -> Result<tg::status::Status> {
		self.status().await
	}

	async fn stop(&self) -> Result<()> {
		self.stop();
		self.join().await?;
		Ok(())
	}

	async fn clean(&self) -> Result<()> {
		self.clean().await
	}

	async fn get_object_exists(&self, id: &tg::object::Id) -> Result<bool> {
		self.get_object_exists(id).await
	}

	async fn get_object(&self, id: &tg::object::Id) -> Result<Bytes> {
		self.get_object(id).await
	}

	async fn try_get_object(&self, id: &tg::object::Id) -> Result<Option<Bytes>> {
		self.try_get_object(id).await
	}

	async fn try_put_object(
		&self,
		id: &tg::object::Id,
		bytes: &Bytes,
	) -> Result<Result<(), Vec<tg::object::Id>>> {
		self.try_put_object(id, bytes).await
	}

	async fn try_get_tracker(&self, path: &Path) -> Result<Option<tg::Tracker>> {
		self.try_get_tracker(path).await
	}

	async fn set_tracker(&self, path: &Path, tracker: &tg::Tracker) -> Result<()> {
		self.set_tracker(path, tracker).await
	}

	async fn try_get_build_for_target(&self, id: &tg::target::Id) -> Result<Option<tg::build::Id>> {
		self.try_get_build_for_target(id).await
	}

	async fn get_or_create_build_for_target(&self, id: &tg::target::Id) -> Result<tg::build::Id> {
		self.get_or_create_build_for_target(id).await
	}

	async fn get_build_from_queue(&self) -> Result<tg::build::Id> {
		self.get_build_from_queue().await
	}

	async fn try_get_build_target(&self, id: &tg::build::Id) -> Result<Option<tg::target::Id>> {
		self.try_get_build_target(id).await
	}

	async fn try_get_build_children(
		&self,
		id: &tg::build::Id,
	) -> Result<Option<BoxStream<'static, Result<tg::build::Id>>>> {
		self.try_get_build_children(id).await
	}

	async fn add_build_child(
		&self,
		build_id: &tg::build::Id,
		child_id: &tg::build::Id,
	) -> Result<()> {
		self.add_build_child(build_id, child_id).await
	}

	async fn try_get_build_log(
		&self,
		id: &tg::build::Id,
	) -> Result<Option<BoxStream<'static, Result<Bytes>>>> {
		self.try_get_build_log(id).await
	}

	async fn add_build_log(&self, build_id: &tg::build::Id, bytes: Bytes) -> Result<()> {
		self.add_build_log(build_id, bytes).await
	}

	async fn try_get_build_result(&self, id: &tg::build::Id) -> Result<Option<Result<tg::Value>>> {
		self.try_get_build_result(id).await
	}

	async fn cancel_build(&self, id: &tg::build::Id) -> Result<()> {
		self.cancel_build(id).await
	}

	async fn finish_build(&self, id: &tg::build::Id, result: Result<tg::Value>) -> Result<()> {
		self.finish_build(id, result).await
	}

	async fn search_packages(&self, query: &str) -> Result<Vec<tg::Package>> {
		self.inner
			.remote
			.as_ref()
			.wrap_err("The server does not have a remote.")?
			.search_packages(query)
			.await
	}

	async fn get_package(&self, name: &str) -> Result<Option<tg::Package>> {
		self.inner
			.remote
			.as_ref()
			.wrap_err("The server does not have a remote.")?
			.get_package(name)
			.await
	}

	async fn get_package_version(
		&self,
		name: &str,
		version: &str,
	) -> Result<Option<tg::artifact::Id>> {
		self.inner
			.remote
			.as_ref()
			.wrap_err("The server does not have a remote.")?
			.get_package_version(name, version)
			.await
	}

	async fn publish_package(&self, token: &str, id: &tg::artifact::Id) -> Result<()> {
		let remote = self
			.inner
			.remote
			.as_ref()
			.wrap_err("The server does not have a remote.")?;
		tg::object::Handle::with_id(id.clone().into())
			.push(self, remote.as_ref())
			.await
			.wrap_err("Failed to push the package.")?;
		remote.publish_package(token, id).await
	}

	async fn get_package_metadata(&self, id: &tg::Id) -> Result<Option<tg::package::Metadata>> {
		// Get the package.
		let package =
			tg::Directory::with_id(id.clone().try_into().wrap_err("Invalid Directory ID.")?);
		let metadata = package.metadata(self).await?;
		Ok(Some(metadata))
	}

	async fn get_package_dependencies(&self, id: &tg::Id) -> Result<Option<Vec<tg::Dependency>>> {
		// Get the package.
		let package =
			tg::Directory::with_id(id.clone().try_into().wrap_err("Invalid Directory ID.")?);
		let dependencies = package.dependencies(self).await?;
		Ok(Some(dependencies))
	}

	async fn create_login(&self) -> Result<tg::user::Login> {
		self.inner
			.remote
			.as_ref()
			.wrap_err("The server does not have a remote.")?
			.create_login()
			.await
	}

	async fn get_login(&self, id: &tg::Id) -> Result<Option<tg::user::Login>> {
		self.inner
			.remote
			.as_ref()
			.wrap_err("The server does not have a remote.")?
			.get_login(id)
			.await
	}

	async fn get_current_user(&self, token: &str) -> Result<Option<tg::user::User>> {
		self.inner
			.remote
			.as_ref()
			.wrap_err("The server does not have a remote.")?
			.get_current_user(token)
			.await
	}
}
