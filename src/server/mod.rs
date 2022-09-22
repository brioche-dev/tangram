use self::{config::Config, lock::Lock};
use crate::{
	client::{self, Client},
	expression::Expression,
	hash::Hash,
	util::path_exists,
};
use anyhow::Result;
use async_recursion::async_recursion;
use async_trait::async_trait;
use futures::future::try_join_all;
use futures::FutureExt;
use hyperlocal::UnixServerExt;
use std::{
	convert::Infallible,
	net::SocketAddr,
	path::{Path, PathBuf},
	sync::Arc,
};

pub mod autoshell;
pub mod blob;
mod checkin;
mod checkout;
pub mod config;
mod db;
mod error;
mod evaluate;
mod evaluators;
pub mod expression;
mod fragment;
mod gc;
mod lock;
mod migrations;
mod package;
mod package_versions;
mod temp;

pub struct Server {
	/// This is the path to the directory where the server stores its data.
	path: PathBuf,

	/// This lock can be used to acquire exclusive and non-exclusive access to the server path as necessary.
	lock: Lock,

	/// This is the connection pool for the server's SQLite database.
	database_connection_pool: deadpool_sqlite::Pool,

	/// These are the server's peers.
	peers: Vec<Client>,

	/// These are the evaluators.
	evaluators: Vec<Box<dyn Send + Sync + Evaluator>>,
}

#[async_trait]
pub trait Evaluator {
	async fn evaluate(
		&self,
		server: &Arc<Server>,
		hash: Hash,
		expression: &Expression,
	) -> Result<Option<Hash>>;
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
		let lock = Lock::new(lock_path);

		// Migrate the path.
		Server::migrate(&path).await?;

		// Create the database pool.
		let database_path = path.join("db.sqlite3");
		let database_connection_pool =
			tokio::task::block_in_place(|| Server::create_database_pool(database_path))?;

		// Create the peer clients.
		let peers = try_join_all(config.peers.into_iter().map(|url| {
			Client::new_with_config(client::config::Config {
				transport: client::config::Transport::Tcp { url },
			})
		}))
		.await?;

		// Create the evaluators.
		let evaluators: Vec<Box<dyn Send + Sync + Evaluator>> = vec![
			Box::new(self::evaluators::array::Array::new()),
			Box::new(self::evaluators::fetch::Fetch::new()),
			Box::new(self::evaluators::map::Map::new()),
			Box::new(self::evaluators::path::Path::new()),
			Box::new(self::evaluators::primitive::Primitive::new()),
			Box::new(self::evaluators::process::Process::new()),
			Box::new(self::evaluators::target::Target::new()),
			Box::new(self::evaluators::template::Template::new()),
		];

		// Create the server.
		let server = Server {
			path,
			lock,
			database_connection_pool,
			peers,
			evaluators,
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
							let response = server.handle_request_wrapper(request).await;
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
							let response = server.handle_request_wrapper(request).await;
							Ok::<_, Infallible>(response)
						}
					}))
				}
			}))
			.await?;
		Ok(())
	}

	pub async fn handle_request_wrapper(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> http::Response<hyper::Body> {
		match self.handle_request(request).await {
			Ok(Some(response)) => response,
			Ok(None) => http::Response::builder()
				.status(http::StatusCode::NOT_FOUND)
				.body(hyper::Body::from("Not found."))
				.unwrap(),
			Err(error) => {
				tracing::error!(?error, backtrace = %error.backtrace());
				http::Response::builder()
					.status(http::StatusCode::INTERNAL_SERVER_ERROR)
					.body(hyper::Body::from(format!("{:?}", error)))
					.unwrap()
			},
		}
	}

	pub async fn handle_request(
		self: &Arc<Self>,
		request: http::Request<hyper::Body>,
	) -> Result<Option<http::Response<hyper::Body>>> {
		let method = request.method().clone();
		let path = request.uri().path().to_owned();
		let path_components = path.split('/').skip(1).collect::<Vec<_>>();
		let response = match (method, path_components.as_slice()) {
			(http::Method::GET, ["autoshells", ""]) => {
				Some(self.handle_get_autoshells_request(request).boxed())
			},
			(http::Method::POST, ["autoshells", ""]) => {
				Some(self.handle_create_autoshell_request(request).boxed())
			},
			(http::Method::DELETE, ["autoshells", ""]) => {
				Some(self.handle_delete_autoshell_request(request).boxed())
			},
			(http::Method::GET, ["blobs", _]) => {
				Some(self.handle_get_blob_request(request).boxed())
			},
			(http::Method::POST, ["blobs", _]) => {
				Some(self.handle_create_blob_request(request).boxed())
			},
			(http::Method::GET, ["expressions", _]) => {
				Some(self.handle_get_expression_request(request).boxed())
			},
			(http::Method::POST, ["expressions", _]) => {
				Some(self.handle_create_expression_request(request).boxed())
			},
			(http::Method::POST, ["expressions", _, "evaluate"]) => {
				Some(self.handle_evaluate_expression_request(request).boxed())
			},
			(http::Method::GET, ["packages"]) => {
				Some(self.handle_get_packages_request(request).boxed())
			},
			(http::Method::GET, ["packages", _]) => {
				Some(self.handle_get_package_request(request).boxed())
			},
			(http::Method::POST, ["packages", _]) => {
				Some(self.handle_create_package_request(request).boxed())
			},
			(http::Method::GET, ["packages", _, "versions", _]) => {
				Some(self.handle_get_package_version_request(request).boxed())
			},
			(http::Method::POST, ["packages", _, "versions", _]) => {
				Some(self.handle_create_package_version_request(request).boxed())
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
