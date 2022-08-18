use self::repl::Repl;
use crate::{
	repl::ReplId,
	server::temp::{Temp, TempId},
	util::path_exists,
};
use anyhow::{Context, Result};
use futures::FutureExt;
use hyperlocal::UnixServerExt;
use std::{
	collections::BTreeMap,
	convert::Infallible,
	net::SocketAddr,
	path::{Path, PathBuf},
	sync::Arc,
};
use tokio::{
	fs,
	sync::{Mutex, RwLock},
};

pub mod artifact;
pub mod blob;
mod checkin;
mod checkout;
pub mod db;
pub mod evaluate;
pub mod fragment;
pub mod migrations;
pub mod object;
pub mod repl;
pub mod runtime;
pub mod temp;

pub struct Server {
	/// This is the path to the directory where the server stores its data.
	path: PathBuf,

	/// This file is held with an advisory lock to ensure only one server has access to the [`path`].
	#[allow(dead_code)]
	path_lock_file: fs::File,

	/// This is the garbage collection lock. Any operation that accesses artifacts, objects, blobs, fragments, or temps must acquire shared access. Garbage collection must acquire exclusive access.
	gc_lock: RwLock<()>,

	/// This is the connection pool for the server's SQLite database.
	database_connection_pool: deadpool_sqlite::Pool,

	/// This local pool handle is for running tasks that must be pinned to a single thread.
	local_pool_handle: tokio_util::task::LocalPoolHandle,

	/// This HTTP client is for performing HTTP requests when running fetch expressions.
	http_client: reqwest::Client,

	/// These are the active temps.
	temps: Mutex<BTreeMap<TempId, Temp>>,

	/// These are the active REPLs.
	repls: Mutex<BTreeMap<ReplId, Repl>>,
}

impl Server {
	pub async fn new(path: impl Into<PathBuf>) -> Result<Arc<Server>> {
		// Ensure the path exists.
		let path = path.into();
		fs::create_dir_all(&path).await?;

		// Acquire a lock to the path.
		let path_lock = Server::acquire_path_lock_file(&path).await?;

		// Migrate the path.
		Server::migrate(&path).await?;

		// Remove any stale temps.
		let temp_path = path.join("temps");
		if path_exists(&temp_path).await? {
			tokio::fs::remove_dir_all(&temp_path).await?;
			tokio::fs::create_dir_all(&temp_path).await?;
		}

		// Create the database pool.
		let database_path = path.join("db.sqlite3");
		let database_connection_pool =
			tokio::task::block_in_place(|| Server::create_database_pool(database_path))?;

		// Create the local task pool.
		let available_parallelism = std::thread::available_parallelism()?.into();
		let local_pool_handle = tokio_util::task::LocalPoolHandle::new(available_parallelism);

		// Create the HTTP client.
		let http_client = reqwest::Client::new();

		// Create the server.
		let server = Server {
			path,
			path_lock_file: path_lock,
			gc_lock: RwLock::new(()),
			database_connection_pool,
			local_pool_handle,
			http_client,
			repls: Mutex::new(BTreeMap::new()),
			temps: Mutex::new(BTreeMap::new()),
		};

		// Remove the socket file if it exists.
		let socket_path = server.path.join("socket");
		if path_exists(&socket_path).await? {
			tokio::fs::remove_file(&socket_path).await?;
		}

		let server = Arc::new(server);

		Ok(server)
	}

	/// Acquire the lock to the server path.
	#[cfg(any(target_os = "linux", target_os = "macos"))]
	async fn acquire_path_lock_file(path: &std::path::Path) -> anyhow::Result<fs::File> {
		use nix::fcntl::{flock, FlockArg};
		use std::os::unix::io::AsRawFd;
		let lock_file = fs::OpenOptions::new()
			.read(true)
			.write(true)
			.create(true)
			.open(path.join("lock"))
			.await?;
		flock(lock_file.as_raw_fd(), FlockArg::LockExclusiveNonblock).context(
			"Failed to acquire the lock to the server path. Is there a tangram server already running?",
		)?;
		Ok(lock_file)
	}

	pub fn path(&self) -> &Path {
		&self.path
	}

	pub async fn serve_unix(self: &Arc<Self>) -> Result<()> {
		let server = Arc::clone(self);
		let path = self.path.join("socket");
		hyper::Server::bind_unix(&path)
			.map(|server| {
				tracing::info!("🚀 Serving at {}.", path.display());
				server
			})?
			.serve(hyper::service::make_service_fn(move |_| {
				let server = Arc::clone(&server);
				async move {
					Ok::<_, Infallible>(hyper::service::service_fn(move |request| {
						let server = Arc::clone(&server);
						async move {
							let response = server.handle_request(request).await;
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
				tracing::info!("🚀 Serving on {}.", addr);
				server
			})?
			.serve(hyper::service::make_service_fn(move |_| {
				let server = Arc::clone(&server);
				async move {
					Ok::<_, Infallible>(hyper::service::service_fn(move |request| {
						let server = Arc::clone(&server);
						async move {
							let response = server.handle_request(request).await;
							Ok::<_, Infallible>(response)
						}
					}))
				}
			}))
			.await?;
		Ok(())
	}

	async fn handle_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> http::Response<hyper::Body> {
		let method = request.method().clone();
		let path = request.uri().path().to_owned();
		let path_components = path.split('/').skip(1).collect::<Vec<_>>();
		let response: Result<http::Response<hyper::Body>> =
			match (method, path_components.as_slice()) {
				// (http::Method::POST, ["blobs", _]) => {
				// 	self.handle_create_blob_request(request).boxed()
				// },
				// (http::Method::GET, ["expressions", _]) => {
				// 	self.handle_expression_request(request).boxed()
				// },
				(http::Method::POST, ["expressions", _, "evaluate"]) => {
					self.handle_evaluate_expression_request(request).boxed()
				},
				// (http::Method::POST, ["objects", _]) => {
				// 	self.handle_create_object_request(request).boxed()
				// },
				(http::Method::POST, ["repls", ""]) => {
					self.handle_create_repl_request(request).boxed()
				},
				(http::Method::POST, ["repls", _, "run"]) => {
					self.handle_repl_run_request(request).boxed()
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

	// async fn handle_expression_request(
	// 	self: &Arc<Self>,
	// 	request: http::Request<hyper::Body>,
	// ) -> Result<http::Response<hyper::Body>> {
	// 	todo!()
	// }

	async fn handle_evaluate_expression_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the request.
		let body = hyper::body::to_bytes(request).await?;
		let expression = serde_json::from_slice(&body)?;

		// Evaluate the expression.
		let value = self.evaluate(expression).await?;

		// Create the response.
		let body = serde_json::to_vec(&value)?;
		let response = http::Response::builder()
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}

	async fn handle_create_repl_request(
		self: &Arc<Self>,
		_request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Create a repl.
		let value = self.create_repl().await?;

		// Create the response.
		let body = serde_json::to_vec(&value)?;
		let response = http::Response::builder()
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}

	async fn handle_repl_run_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> Result<http::Response<hyper::Body>> {
		// Read the request.
		let body = hyper::body::to_bytes(request).await?;
		let repl_run_request: self::repl::RunRequest = serde_json::from_slice(&body)?;

		// Run the repl.
		let output = self
			.repl_run(&repl_run_request.repl_id, repl_run_request.code)
			.await?;

		// Create the response.
		let output = match output {
			Ok(output) => output,
			Err(message) => Some(message),
		};
		let body = serde_json::to_vec(&output)?;
		let response = http::Response::builder()
			.body(hyper::Body::from(body))
			.unwrap();

		Ok(response)
	}
}
