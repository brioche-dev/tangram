use self::progress::Progress;
use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::BoxStream;
use std::{
	collections::HashMap,
	os::fd::AsRawFd,
	path::{Path, PathBuf},
	sync::{Arc, Weak},
};
use tangram_client as tg;
use tg::{return_error, util::rmrf, Result, Wrap, WrapErr};

mod build;
mod clean;
mod migrations;
mod object;
mod progress;
mod tracker;

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

	/// The server's version.
	version: String,

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
	trackers: lmdb::Database,
}

pub struct Options {
	pub parent: Option<Box<dyn tg::Client>>,
	pub path: PathBuf,
	pub version: String,
}

impl Server {
	pub async fn new(options: Options) -> Result<Server> {
		let Options {
			parent,
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
			trackers,
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
			version,
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

	async fn status(&self) -> Result<tg::status::Status> {
		Ok(tg::status::Status {
			version: self.inner.version.clone(),
		})
	}

	async fn stop(&self) -> Result<()> {
		std::process::exit(0);
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

	fn file_descriptor_semaphore(&self) -> &tokio::sync::Semaphore {
		&self.inner.file_descriptor_semaphore
	}

	async fn status(&self) -> Result<tg::status::Status> {
		self.status().await
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

	async fn try_get_build_queue_item(&self) -> Result<Option<tg::build::Id>> {
		Ok(None)
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
		_build_id: &tg::build::Id,
		_child_id: &tg::build::Id,
	) -> Result<()> {
		return_error!("This server does not support builders.");
	}

	async fn try_get_build_log(
		&self,
		id: &tg::build::Id,
	) -> Result<Option<BoxStream<'static, Result<Bytes>>>> {
		self.try_get_build_log(id).await
	}

	async fn add_build_log(&self, _build_id: &tg::build::Id, _bytes: Bytes) -> Result<()> {
		return_error!("This server does not support builders.");
	}

	async fn try_get_build_result(&self, id: &tg::build::Id) -> Result<Option<Result<tg::Value>>> {
		self.try_get_build_result(id).await
	}

	async fn set_build_result(
		&self,
		_build_id: &tg::build::Id,
		_result: Result<tg::Value>,
	) -> Result<()> {
		return_error!("This server does not support builders.");
	}

	async fn finish_build(&self, _id: &tg::build::Id) -> Result<()> {
		return_error!("This server does not support builders.");
	}

	async fn search_packages(&self, query: &str) -> Result<Vec<tg::Package>> {
		self.inner
			.parent
			.as_ref()
			.wrap_err("The server does not have a parent.")?
			.search_packages(query)
			.await
	}

	async fn get_package(&self, name: &str) -> Result<Option<tg::Package>> {
		self.inner
			.parent
			.as_ref()
			.wrap_err("The server does not have a parent.")?
			.get_package(name)
			.await
	}

	async fn get_package_version(
		&self,
		name: &str,
		version: &str,
	) -> Result<Option<tg::artifact::Id>> {
		self.inner
			.parent
			.as_ref()
			.wrap_err("The server does not have a parent.")?
			.get_package_version(name, version)
			.await
	}

	async fn publish_package(&self, token: &str, id: &tg::artifact::Id) -> Result<()> {
		let parent = self
			.inner
			.parent
			.as_ref()
			.wrap_err("The server does not have a parent.")?;
		tg::object::Handle::with_id(id.clone().into())
			.push(self, parent.as_ref())
			.await
			.wrap_err("Failed to push the package.")?;
		parent.publish_package(token, id).await
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

	async fn get_current_user(&self, token: &str) -> Result<Option<tg::user::User>> {
		self.inner
			.parent
			.as_ref()
			.wrap_err("The server does not have a parent.")?
			.get_current_user(token)
			.await
	}
}
