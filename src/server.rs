use crate::{id, task, Client, Error, Result};
use futures::FutureExt;
use http_body_util::BodyExt;
use itertools::Itertools;
use std::{
	collections::HashMap,
	convert::Infallible,
	net::SocketAddr,
	path::{Path, PathBuf},
	sync::Arc,
};
use url::Url;

mod object;
mod run;

/// A server.
#[derive(Clone, Debug)]
pub struct Server {
	pub(crate) state: Arc<State>,
}

#[derive(Debug)]
pub struct State {
	/// The database.
	pub(crate) database: Database,

	/// A semaphore that prevents opening too many file descriptors.
	pub(crate) file_descriptor_semaphore: tokio::sync::Semaphore,

	/// An HTTP client for downloading resources.
	pub(crate) http_client: reqwest::Client,

	/// A local pool for running JS tasks.
	pub(crate) local_pool: tokio_util::task::LocalPoolHandle,

	/// The options the server was created with.
	pub(crate) options: Options,

	/// A client for communicating with the parent.
	pub(crate) parent: Option<Client>,

	/// The path to the directory where the server stores its data.
	pub(crate) path: PathBuf,

	/// A semaphore that limits the number of concurrent subprocesses.
	pub(crate) process_semaphore: tokio::sync::Semaphore,

	/// The state of the server's running tasks.
	pub(crate) running: std::sync::RwLock<(
		HashMap<task::Id, crate::run::Id, id::BuildHasher>,
		HashMap<crate::run::Id, Arc<crate::run::State>, id::BuildHasher>,
	)>,
}

#[derive(Debug)]
pub struct Database {
	pub(crate) env: lmdb::Environment,
	pub(crate) objects: lmdb::Database,
	pub(crate) assignments: lmdb::Database,
}

#[derive(Clone, Debug, Default)]
pub struct Options {
	pub parent_token: Option<String>,
	pub parent_url: Option<Url>,
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
		let objects = env.open_db(Some("objects"))?;
		let assignments = env.open_db(Some("assignments"))?;
		let database = Database {
			env,
			objects,
			assignments,
		};

		// Create the file system semaphore.
		let file_descriptor_semaphore = tokio::sync::Semaphore::new(16);

		// Create the HTTP client for downloading resources.
		let http_client = reqwest::Client::new();

		// Create the local pool for running JS tasks.
		let local_pool = tokio_util::task::LocalPoolHandle::new(
			std::thread::available_parallelism().unwrap().get(),
		);

		// Create the parent client.
		let parent = if let Some(url) = options.parent_url.as_ref() {
			let token = options.parent_token.clone();
			Some(Client::with_url(url.clone(), token))
		} else {
			None
		};

		// Create the process semaphore.
		let process_semaphore =
			tokio::sync::Semaphore::new(std::thread::available_parallelism().unwrap().get());

		// Create the state of the server's running tasks.
		let running = std::sync::RwLock::new((HashMap::default(), HashMap::default()));

		// Create the state.
		let state = Arc::new(State {
			database,
			file_descriptor_semaphore,
			http_client,
			local_pool,
			options,
			parent,
			path,
			process_semaphore,
			running,
		});

		// Create the server.
		let server = Server { state };

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

	pub async fn serve(self, addr: SocketAddr) -> Result<()> {
		let listener = tokio::net::TcpListener::bind(&addr)
			.await
			.map_err(Error::with_error)?;
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
			Some(response.await.map_err(Error::with_error)?)
		} else {
			None
		};
		Ok(response)
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
