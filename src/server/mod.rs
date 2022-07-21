use crate::{
	artifact::{Artifact, ArtifactHash},
	id::Id,
	object::ObjectHash,
};
use anyhow::{Context, Result};
use futures::FutureExt;
use hyperlocal::UnixServerExt;
use std::{collections::BTreeMap, convert::Infallible, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{
	fs,
	sync::{Mutex, RwLock},
};

mod evaluate;
pub mod graphql;
mod migrations;
mod repl;
pub mod runtime;

pub enum Bind {
	Unix(PathBuf),
	Tcp(SocketAddr),
}

pub struct Server {
	/// This is the path where the server stores its data.
	path: PathBuf,

	/// This file is held with an advisory lock to ensure only one server has access to the [`path`].
	_lock_file: fs::File,

	/// This is the global build lock. Builds must acquire shared access. Garbage collection must acquire exclusive access.
	lock: RwLock<()>,

	_database_pool: sqlx::sqlite::SqlitePool,

	/// This tokio task pool is for running js tasks, which need to be local to a single thread.
	local_pool_handle: tokio_util::task::LocalPoolHandle,

	/// This reqwest client is for performing HTTP requests when fetching.
	http_client: reqwest::Client,

	/// This is the schema for the graphql server.
	schema: Arc<self::graphql::Schema>,

	repls: Mutex<BTreeMap<Id, runtime::js::Runtime>>,
}

impl Server {
	/// Get the server's path.
	#[must_use]
	pub fn path(&self) -> PathBuf {
		self.path.clone()
	}

	// /// Get the path to the checkouts directory.
	// #[must_use]
	// fn database_path(&self) -> PathBuf {
	// 	self.path.join("db.sqlite3")
	// }

	// /// Get the path to the checkouts directory.
	// #[must_use]
	// fn blobs_path(&self) -> PathBuf {
	// 	self.path.join("blobs")
	// }

	// /// Get the path to the checkouts directory.
	// #[must_use]
	// fn checkouts_path(&self) -> PathBuf {
	// 	self.path.join("checkouts")
	// }

	// /// Get the path to the fragments directory.
	// #[must_use]
	// fn fragments_path(&self) -> PathBuf {
	// 	self.path.join("fragments")
	// }

	/// Get the path to the socket.
	#[must_use]
	fn socket_path(&self) -> PathBuf {
		self.path.join("socket")
	}

	pub async fn new(path: impl Into<PathBuf>) -> Result<Arc<Server>> {
		let path = path.into();
		fs::create_dir_all(&path).await?;

		// Acquire a lock to the path.
		let lock_path = path.join("lock");
		let lock_file = Server::acquire_lock_file(&lock_path).await?;

		// Ensure the top levels directories exist.
		let artifacts_path = path.join("artifacts");
		fs::create_dir_all(&artifacts_path).await?;
		let fragments_path = path.join("fragments");
		fs::create_dir_all(&fragments_path).await?;

		// Remove the socket file if it exists.
		let socket_path = path.join("socket");
		let socket_path_exists = match tokio::fs::metadata(&socket_path).await {
			Ok(_) => true,
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
			Err(error) => return Err(error.into()),
		};
		if socket_path_exists {
			tokio::fs::remove_file(&socket_path).await?;
		}

		// Create the database pool.
		let database_path = path.join("db.sqlite3");
		let database_connect_options = sqlx::sqlite::SqliteConnectOptions::new()
			.filename(database_path)
			.create_if_missing(true)
			.foreign_keys(true)
			.journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
			.shared_cache(true);
		let database_pool = sqlx::sqlite::SqlitePoolOptions::new()
			.connect_with(database_connect_options)
			.await?;
		if migrations::empty(&database_pool).await? {
			// Run all migrations if the database is empty.
			migrations::run(&database_pool).await?;
		} else {
			// If the database is not empty, verify that all migrations have already been run.
			migrations::verify(&database_pool).await?;
		}

		// Create the local task pool.
		let local_pool_handle = tokio_util::task::LocalPoolHandle::new(num_cpus::get());

		// Create the HTTP client.
		let http_client = reqwest::Client::new();

		let schema = Arc::new(self::graphql::Schema::new(
			self::graphql::Query,
			self::graphql::Mutation,
			juniper::EmptySubscription::new(),
		));

		let server = Server {
			path,
			_lock_file: lock_file,
			lock: RwLock::new(()),
			_database_pool: database_pool,
			local_pool_handle,
			http_client,
			schema,
			repls: Mutex::new(BTreeMap::new()),
		};
		let server = Arc::new(server);

		Ok(server)
	}

	/// Acquire the lock to the root path.
	#[cfg(any(target_os = "linux", target_os = "macos"))]
	async fn acquire_lock_file(lock_path: &std::path::Path) -> anyhow::Result<fs::File> {
		use nix::fcntl::{flock, FlockArg};
		use std::os::unix::io::AsRawFd;
		let lock_file = fs::OpenOptions::new()
			.read(true)
			.write(true)
			.create(true)
			.open(lock_path)
			.await?;
		flock(lock_file.as_raw_fd(), FlockArg::LockExclusiveNonblock).with_context(|| {
			let lock_path = lock_path.display();
			format!(
				"Failed to acquire the lock file at {}. Is there a tangram server already running?",
				lock_path,
			)
		})?;
		Ok(lock_file)
	}

	pub async fn create_artifact(
		_object_hash: ObjectHash,
		_dependencies: Vec<ArtifactHash>,
	) -> Result<Artifact> {
		todo!()
	}

	pub async fn create_artifact_from_reader<R>(&self, _reader: &mut R) -> Result<Artifact> {
		todo!()
	}

	pub async fn serve_unix(self: &Arc<Self>) -> Result<()> {
		let server = Arc::clone(self);
		let path = server.socket_path();
		hyper::Server::bind_unix(&path)
			.map(|server| {
				tracing::info!("🚀 serving at {}", path.display());
				server
			})?
			.serve(hyper::service::make_service_fn(move |_| {
				let server = Arc::clone(&server);
				async move {
					Ok::<_, Infallible>(hyper::service::service_fn(move |request| {
						let server = Arc::clone(&server);
						async move {
							let response = server.handle(request).await;
							Ok::<_, Infallible>(response)
						}
					}))
				}
			}))
			.await?;
		Ok(())
	}

	pub async fn serve_tcp(self: &Arc<Self>, addr: SocketAddr) -> Result<()> {
		let server = Arc::clone(self);
		hyper::Server::try_bind(&addr)
			.map(|server| {
				tracing::info!("🚀 serving on {}", addr);
				server
			})?
			.serve(hyper::service::make_service_fn(move |_| {
				let server = Arc::clone(&server);
				async move {
					Ok::<_, Infallible>(hyper::service::service_fn(move |request| {
						let server = Arc::clone(&server);
						async move {
							let response = server.handle(request).await;
							Ok::<_, Infallible>(response)
						}
					}))
				}
			}))
			.await?;
		Ok(())
	}

	async fn handle(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> http::Response<hyper::Body> {
		let method = request.method().clone();
		let path = request.uri().path().to_owned();
		let path_components = path.split('/').skip(1).collect::<Vec<_>>();
		let response: Result<http::Response<hyper::Body>> =
			match (method, path_components.as_slice()) {
				(http::Method::GET, ["graphiql"]) => {
					juniper_hyper::graphiql("/graphql", None).map(Ok).boxed()
				},
				(http::Method::GET | http::Method::POST, ["graphql"]) => {
					let schema = Arc::clone(&self.schema);
					let context = Arc::new(Arc::clone(self));
					juniper_hyper::graphql(schema, context, request)
						.map(Ok)
						.boxed()
				},
				(_, _) => {
					let response = http::Response::builder()
						.status(http::StatusCode::NOT_FOUND)
						.body(hyper::Body::from("Not found."))
						.unwrap();
					let response = Ok(response);
					std::future::ready(response).boxed()
				},
			}
			.await;
		response.unwrap_or_else(|error| {
			tracing::error!(%error, backtrace = %error.backtrace());
			http::Response::builder()
				.status(http::StatusCode::INTERNAL_SERVER_ERROR)
				.body(hyper::Body::from("Internal server error."))
				.unwrap()
		})
	}
}
