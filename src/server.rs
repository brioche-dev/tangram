use crate::{client, id, package, target, Client, Id, Result, Value};
use async_trait::async_trait;
use futures::{stream::BoxStream, FutureExt};
use http_body_util::BodyExt;
use itertools::Itertools;
use std::{
	collections::HashMap,
	convert::Infallible,
	net::SocketAddr,
	path::{Path, PathBuf},
	sync::Arc,
};

pub mod build;
pub mod object;

/// A server.
#[derive(Clone, Debug)]
pub struct Server {
	state: Arc<State>,
}

#[derive(Debug)]
pub struct State {
	/// The database.
	database: Database,

	/// A semaphore that prevents opening too many file descriptors.
	file_descriptor_semaphore: tokio::sync::Semaphore,

	/// A local pool for running JS builds.
	local_pool: tokio_util::task::LocalPoolHandle,

	/// A client for communicating with the parent.
	parent: Option<Box<dyn Client>>,

	/// The path to the directory where the server stores its data.
	path: PathBuf,

	/// The state of the server's running builds.
	running: std::sync::RwLock<(BuildForTargetMap, BuildStateMap)>,
	//
	// /// The VFS server task.
	// vfs_server_task: std::sync::Mutex<Option<tokio::task::JoinHandle<Result<()>>>>,
}

type BuildForTargetMap = HashMap<target::Id, crate::build::Id, id::BuildHasher>;

type BuildStateMap = HashMap<crate::build::Id, Arc<self::build::State>, id::BuildHasher>;

#[derive(Debug)]
pub struct Database {
	pub(crate) env: lmdb::Environment,
	pub(crate) objects: lmdb::Database,
	pub(crate) assignments: lmdb::Database,
}

impl Server {
	pub async fn new(path: PathBuf, parent: Option<Box<dyn Client>>) -> Result<Server> {
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

		// Create the file system semaphore.
		let file_descriptor_semaphore = tokio::sync::Semaphore::new(16);

		// Create the local pool for running JS builds.
		let local_pool = tokio_util::task::LocalPoolHandle::new(
			std::thread::available_parallelism().unwrap().get(),
		);

		// Create the state of the server's running builds.
		let running = std::sync::RwLock::new((HashMap::default(), HashMap::default()));

		// Create the VFS server task.
		// let vfs_server_task = std::sync::Mutex::new(None);

		// Create the state.
		let state = Arc::new(State {
			database,
			file_descriptor_semaphore,
			local_pool,
			parent,
			path,
			running,
			// vfs_server_task,
		});

		// Create the server.
		let server = Server { state };

		// // Start the VFS server.
		// let kind = if cfg!(target_os = "linux") {
		// 	vfs::Kind::Fuse
		// } else {
		// 	vfs::Kind::Nfs(2049)
		// };

		// // Mount the VFS server.
		// let task = vfs::Server::new(kind, client)
		// 	.mount(server.artifacts_path())
		// 	.await?;
		// server.state.vfs_server_task.lock().unwrap().replace(task);

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
		let listener = tokio::net::TcpListener::bind(&addr).await?;
		tracing::info!("🚀 Serving on {}.", addr);
		loop {
			let (stream, _) = listener.accept().await?;
			let stream = hyper_util::rt::TokioIo::new(stream);
			let server = self.clone();
			tokio::spawn(async move {
				hyper::server::conn::http2::Builder::new(hyper_util::rt::TokioExecutor::new())
					.serve_connection(
						stream,
						hyper::service::service_fn(move |request| {
							let server = server.clone();
							async move {
								let response = server.handle_request(request).await;
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
		match self.handle_request_inner(request).await {
			Ok(Some(response)) => response,
			Ok(None) => http::Response::builder()
				.status(http::StatusCode::NOT_FOUND)
				.body(full("Not found."))
				.unwrap(),
			Err(error) => {
				tracing::error!(?error);
				http::Response::builder()
					.status(http::StatusCode::INTERNAL_SERVER_ERROR)
					.body(full("Internal server error."))
					.unwrap()
			},
		}
	}

	async fn handle_request_inner(
		&self,
		request: http::Request<Incoming>,
	) -> Result<Option<http::Response<Outgoing>>> {
		let method = request.method().clone();
		let path = request.uri().path().to_owned();
		let path_components = path.split('/').skip(1).collect_vec();
		let response = match (method, path_components.as_slice()) {
			(http::Method::HEAD, ["v1", "objects", _]) => {
				Some(self.handle_head_object_request(request).boxed())
			},
			(http::Method::GET, ["v1", "objects", _]) => {
				Some(self.handle_get_object_request(request).boxed())
			},
			(http::Method::PUT, ["v1", "objects", _]) => {
				Some(self.handle_put_object_request(request).boxed())
			},
			(_, _) => None,
		};
		let response = if let Some(response) = response {
			Some(response.await?)
		} else {
			None
		};
		Ok(response)
	}
}

#[async_trait]
impl Client for Server {
	fn clone_box(&self) -> Box<dyn Client> {
		Box::new(self.clone())
	}

	fn path(&self) -> Option<&Path> {
		Some(self.path())
	}

	fn set_token(&self, _token: Option<String>) {}

	fn file_descriptor_semaphore(&self) -> &tokio::sync::Semaphore {
		&self.state.file_descriptor_semaphore
	}

	async fn get_object_exists(&self, id: crate::object::Id) -> Result<bool> {
		self.get_object_exists(id).await
	}

	async fn get_object_bytes(&self, id: crate::object::Id) -> Result<Vec<u8>> {
		self.get_object_bytes(id).await
	}

	async fn try_get_object_bytes(&self, id: crate::object::Id) -> Result<Option<Vec<u8>>> {
		self.try_get_object_bytes(id).await
	}

	async fn try_put_object_bytes(
		&self,
		id: crate::object::Id,
		bytes: &[u8],
	) -> Result<Result<(), Vec<crate::object::Id>>> {
		self.try_put_object_bytes(id, bytes).await
	}

	async fn try_get_build_for_target(
		&self,
		id: crate::target::Id,
	) -> Result<Option<crate::build::Id>> {
		self.try_get_build_for_target(id).await
	}

	async fn get_or_create_build_for_target(
		&self,
		id: crate::target::Id,
	) -> Result<crate::build::Id> {
		self.get_or_create_build_for_target(id).await
	}

	async fn try_get_build_children(
		&self,
		id: crate::build::Id,
	) -> Result<Option<BoxStream<'static, crate::build::Id>>> {
		self.try_get_build_children(id).await
	}

	async fn try_get_build_log(
		&self,
		id: crate::build::Id,
	) -> Result<Option<BoxStream<'static, Vec<u8>>>> {
		self.try_get_build_log(id).await
	}

	async fn try_get_build_output(&self, id: crate::build::Id) -> Result<Option<Option<Value>>> {
		self.try_get_build_output(id).await
	}

	async fn clean(&self) -> Result<()> {
		self.clean().await
	}

	async fn create_login(&self) -> Result<crate::client::Login> {
		todo!()
	}

	async fn get_login(&self, _id: crate::Id) -> Result<Option<crate::client::Login>> {
		todo!()
	}

	async fn publish_package(&self, _id: crate::package::Id) -> Result<()> {
		todo!()
	}

	async fn search_packages(&self, _query: &str) -> Result<Vec<crate::client::SearchResult>> {
		todo!()
	}

	async fn get_current_user(&self) -> Result<crate::client::User> {
		todo!()
	}
}

pub type Incoming = hyper::body::Incoming;
pub type Outgoing = http_body_util::combinators::BoxBody<
	::bytes::Bytes,
	Box<dyn std::error::Error + Send + Sync + 'static>,
>;

/// An empty response body.
#[must_use]
pub fn empty() -> Outgoing {
	http_body_util::Empty::new()
		.map_err(|_| unreachable!())
		.boxed()
}

/// A full response body.
#[must_use]
pub fn full(chunk: impl Into<::bytes::Bytes>) -> Outgoing {
	http_body_util::Full::new(chunk.into())
		.map_err(|_| unreachable!())
		.boxed()
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
