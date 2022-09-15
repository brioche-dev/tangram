use self::{config::Config, lock::Lock};
use crate::{
	client::{self, Client},
	util::path_exists,
};
use anyhow::Result;
use async_recursion::async_recursion;
use futures::future::try_join_all;
use futures::FutureExt;
use hyperlocal::UnixServerExt;
use std::{
	convert::Infallible,
	net::SocketAddr,
	path::{Path, PathBuf},
	sync::Arc,
};

pub mod blob;
mod checkin;
mod checkout;
pub mod config;
pub mod db;
mod error;
pub mod evaluate;
pub mod expression;
pub mod fragment;
mod gc;
mod lock;
pub mod migrations;
mod package;
mod package_versions;
pub mod runtime;
pub mod temp;

pub struct Server {
	/// This is the path to the directory where the server stores its data.
	path: PathBuf,

	/// We use a file with an advisory lock to ensure exclusive and non-exclusive access to the server path as necessary.
	lock: Lock,

	/// This is the connection pool for the server's SQLite database.
	database_connection_pool: deadpool_sqlite::Pool,

	/// This local pool handle is for running tasks that must be pinned to a single thread.
	local_pool_handle: tokio_util::task::LocalPoolHandle,

	/// This HTTP client is for performing HTTP requests when running fetch expressions.
	http_client: reqwest::Client,

	/// These are the peers.
	peers: Vec<Client>,
}

impl Server {
	#[async_recursion]
	#[must_use]
	pub async fn new(config: Config) -> Result<Arc<Server>> {
		// Ensure the path exists.
		let path = config.path;
		tokio::fs::create_dir_all(&path).await?;

		// Create the lock.
		let lock_path = path.join("lock");
		let lock = Lock::new(lock_path).await?;

		// Migrate the path.
		Server::migrate(&path).await?;

		// Create the database pool.
		let database_path = path.join("db.sqlite3");
		let database_connection_pool =
			tokio::task::block_in_place(|| Server::create_database_pool(database_path))?;

		// Create the local task pool.
		let available_parallelism = std::thread::available_parallelism()?.into();
		let local_pool_handle = tokio_util::task::LocalPoolHandle::new(available_parallelism);

		// Create the HTTP client.
		let http_client = reqwest::Client::new();

		// // Create the peer Clients.
		let peers = try_join_all(config.peers.into_iter().map(|url| {
			Client::new_with_config(client::config::Config {
				transport: client::config::Transport::Tcp { url },
			})
		}))
		.await?;

		// Create the server.
		let server = Server {
			path,
			lock,
			database_connection_pool,
			local_pool_handle,
			http_client,
			peers,
		};

		// Remove the socket file if it exists.
		let socket_path = server.path.join("socket");
		if path_exists(&socket_path).await? {
			tokio::fs::remove_file(&socket_path).await?;
		}

		// Wrap the server in an Arc.
		let server = Arc::new(server);

		Ok(server)
	}

	pub fn path(&self) -> &Path {
		&self.path
	}
}

impl Server {
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
				(http::Method::GET, ["blobs", _]) => self.handle_get_blob_request(request).boxed(),
				(http::Method::POST, ["blobs", _]) => {
					self.handle_create_blob_request(request).boxed()
				},
				(http::Method::GET, ["expressions", _]) => {
					self.handle_get_expression_request(request).boxed()
				},
				(http::Method::POST, ["expressions", _]) => {
					self.handle_create_expression_request(request).boxed()
				},
				(http::Method::POST, ["expressions", _, "evaluate"]) => {
					self.handle_evaluate_expression_request(request).boxed()
				},
				// (http::Method::GET, ["packages"]) => {
				// 	self.handle_get_packages_request(request).boxed()
				// },
				// (http::Method::GET, ["packages", _]) => {
				// 	self.handle_get_package_request(request).boxed()
				// },
				// (http::Method::POST, ["packages", _]) => {
				// 	self.handle_create_package_request(request).boxed()
				// },
				// (http::Method::GET, ["packages", _, "versions", _]) => {
				// 	self.handle_get_package_version_request(request).boxed()
				// },
				// (http::Method::POST, ["packages", _, "versions", _]) => {
				// 	self.handle_create_package_version_request(request).boxed()
				// },
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
			tracing::error!(?error, backtrace = %error.backtrace());
			http::Response::builder()
				.status(http::StatusCode::INTERNAL_SERVER_ERROR)
				.body(hyper::Body::from(format!("{:?}", error)))
				.unwrap()
		})
	}
}
