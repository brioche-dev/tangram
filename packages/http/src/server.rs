use crate::{
	net::Addr,
	util::{bad_request, empty, full, get_token, not_found, ok, unauthorized, Incoming, Outgoing},
};
use bytes::Bytes;
use futures::{
	future::{self, BoxFuture},
	FutureExt, TryStreamExt,
};
use http_body_util::{BodyExt, StreamBody};
use hyper_util::rt::TokioIo;
use itertools::Itertools;
use std::{convert::Infallible, path::PathBuf, sync::Arc};
use tangram_client as tg;
use tangram_error::{return_error, Result, WrapErr};
use tokio::net::{TcpListener, UnixListener};
use tokio_util::either::Either;

#[derive(Clone)]
pub struct Server {
	inner: Arc<Inner>,
}

struct Inner {
	client: Box<dyn tg::Client>,
	handler: Option<Handler>,
	task: Task,
}

type Task = (
	std::sync::Mutex<Option<tokio::task::JoinHandle<Result<()>>>>,
	std::sync::Mutex<Option<tokio::task::AbortHandle>>,
);

type Handler = Box<
	dyn Fn(http::Request<Incoming>) -> BoxFuture<'static, Option<Result<http::Response<Outgoing>>>>
		+ Send
		+ Sync
		+ 'static,
>;

impl Server {
	pub async fn start(
		client: &dyn tg::Client,
		addr: Addr,
		handler: Option<Handler>,
	) -> Result<Self> {
		let task = (std::sync::Mutex::new(None), std::sync::Mutex::new(None));
		let inner = Inner {
			client: client.clone_box(),
			handler,
			task,
		};
		let server = Self {
			inner: Arc::new(inner),
		};
		let task = tokio::spawn({
			let server = server.clone();
			async move { server.serve(addr).await }
		});
		let abort = task.abort_handle();
		server.inner.task.0.lock().unwrap().replace(task);
		server.inner.task.1.lock().unwrap().replace(abort);
		Ok(server)
	}

	pub fn stop(&self) {
		if let Some(handle) = self.inner.task.1.lock().unwrap().as_ref() {
			handle.abort();
		};
	}

	pub async fn join(&self) -> Result<()> {
		// Join the task.
		let task = self.inner.task.0.lock().unwrap().take();
		if let Some(task) = task {
			match task.await {
				Ok(result) => Ok(result),
				Err(error) if error.is_cancelled() => Ok(Ok(())),
				Err(error) => Err(error),
			}
			.unwrap()?;
		}

		Ok(())
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
			let connection =
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
					);
			tokio::spawn(async move { connection.await.ok() });
		}
	}

	async fn handle_request(&self, request: http::Request<Incoming>) -> http::Response<Outgoing> {
		let method = request.method().clone();
		let path_components = request.uri().path().split('/').skip(1).collect_vec();
		let response = match (method, path_components.as_slice()) {
			// Server
			(http::Method::GET, ["v1", "status"]) => {
				self.handle_get_status_request(request).map(Some).boxed()
			},
			(http::Method::POST, ["v1", "stop"]) => {
				self.handle_post_stop_request(request).map(Some).boxed()
			},
			(http::Method::POST, ["v1", "clean"]) => {
				self.handle_post_clean_request(request).map(Some).boxed()
			},

			// Builds
			(http::Method::GET, ["v1", "builds", "queue"]) => self
				.handle_get_build_queue_item_request(request)
				.map(Some)
				.boxed(),
			(http::Method::GET, ["v1", "targets", _, "build"]) => self
				.handle_get_build_for_target_request(request)
				.map(Some)
				.boxed(),
			(http::Method::POST, ["v1", "targets", _, "build"]) => self
				.handle_get_or_create_build_for_target_request(request)
				.map(Some)
				.boxed(),
			(http::Method::GET, ["v1", "builds", _, "target"]) => self
				.handle_get_build_target_request(request)
				.map(Some)
				.boxed(),
			(http::Method::GET, ["v1", "builds", _, "children"]) => self
				.handle_get_build_children_request(request)
				.map(Some)
				.boxed(),
			(http::Method::POST, ["v1", "builds", _, "children"]) => self
				.handle_post_build_child_request(request)
				.map(Some)
				.boxed(),
			(http::Method::GET, ["v1", "builds", _, "log"]) => {
				self.handle_get_build_log_request(request).map(Some).boxed()
			},
			(http::Method::POST, ["v1", "builds", _, "log"]) => self
				.handle_post_build_log_request(request)
				.map(Some)
				.boxed(),
			(http::Method::GET, ["v1", "builds", _, "result"]) => self
				.handle_get_build_result_request(request)
				.map(Some)
				.boxed(),
			(http::Method::POST, ["v1", "builds", _, "result"]) => self
				.handle_post_build_result_request(request)
				.map(Some)
				.boxed(),
			(http::Method::POST, ["v1", "builds", _, "cancel"]) => self
				.handle_post_build_cancel_request(request)
				.map(Some)
				.boxed(),
			(http::Method::POST, ["v1", "builds", _, "finish"]) => self
				.handle_post_build_finish_request(request)
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

			// Packages
			(http::Method::GET, ["v1", "registry", "packages", "search"]) => self
				.handle_search_packages_request(request)
				.map(Some)
				.boxed(),
			(http::Method::GET, ["v1", "registry", "packages", _]) => {
				self.handle_get_package_request(request).map(Some).boxed()
			},
			(http::Method::GET, ["v1", "registry", "packages", _, "versions", _]) => self
				.handle_get_package_version_request(request)
				.map(Some)
				.boxed(),
			(http::Method::POST, ["v1", "registry", "packages"]) => self
				.handle_publish_package_request(request)
				.map(Some)
				.boxed(),
			(http::Method::GET, ["v1", "packages", _, "metadata"]) => self
				.handle_get_package_metadata_request(request)
				.map(Some)
				.boxed(),
			(http::Method::GET, ["v1", "packages", _, "dependencies"]) => self
				.handle_get_package_dependencies_request(request)
				.map(Some)
				.boxed(),

			// Trackers
			(http::Method::GET, ["v1", "trackers", _]) => {
				self.handle_get_tracker_request(request).map(Some).boxed()
			},
			(http::Method::PATCH, ["v1", "trackers", _]) => {
				self.handle_patch_tracker_request(request).map(Some).boxed()
			},

			// Users
			(http::Method::POST, ["v1", "logins"]) => {
				self.handle_create_login_request(request).map(Some).boxed()
			},
			(http::Method::GET, ["v1", "logins", _]) => {
				self.handle_get_login_request(request).map(Some).boxed()
			},
			(http::Method::GET, ["v1", "user"]) => self
				.handle_get_current_user_request(request)
				.map(Some)
				.boxed(),

			(_, _) => {
				if let Some(handler) = self.inner.handler.as_ref() {
					handler(request).boxed()
				} else {
					future::ready(None).boxed()
				}
			},
		}
		.await;
		match response {
			None => http::Response::builder()
				.status(http::StatusCode::NOT_FOUND)
				.body(full("Not found."))
				.unwrap(),
			Some(Err(error)) => {
				let trace = error.trace();
				tracing::error!(%trace);
				http::Response::builder()
					.status(http::StatusCode::INTERNAL_SERVER_ERROR)
					.body(full("Internal server error."))
					.unwrap()
			},
			Some(Ok(response)) => response,
		}
	}

	async fn handle_get_status_request(
		&self,
		_request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		let status = self.inner.client.status().await?;
		let body = serde_json::to_vec(&status).unwrap();
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(body))
			.unwrap();
		Ok(response)
	}

	async fn handle_post_stop_request(
		&self,
		_request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		self.inner.client.stop().await?;
		Ok(ok())
	}

	pub async fn handle_post_clean_request(
		&self,
		_request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		self.inner.client.clean().await?;
		Ok(http::Response::builder()
			.status(http::StatusCode::OK)
			.body(empty())
			.unwrap())
	}

	pub async fn handle_get_build_queue_item_request(
		&self,
		_request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		let build_id = self.inner.client.get_build_from_queue().await?;

		// Create the response.
		let body = serde_json::to_vec(&build_id).wrap_err("Failed to serialize the ID.")?;
		let response = http::Response::builder().body(full(body)).unwrap();
		Ok(response)
	}

	pub async fn handle_get_build_for_target_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "targets", id, "build"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let id = id.parse().wrap_err("Failed to parse the ID.")?;

		// Attempt to get the build for the target.
		let Some(build_id) = self.inner.client.try_get_build_for_target(&id).await? else {
			return Ok(not_found());
		};

		// Create the response.
		let body = serde_json::to_vec(&build_id).wrap_err("Failed to serialize the response.")?;
		let response = http::Response::builder().body(full(body)).unwrap();
		Ok(response)
	}

	pub async fn handle_get_or_create_build_for_target_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "targets", id, "build"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let id = id.parse().wrap_err("Failed to parse the ID.")?;

		// Get or create the build for the target.
		let build_id = self
			.inner
			.client
			.get_or_create_build_for_target(&id)
			.await?;

		// Create the response.
		let body = serde_json::to_vec(&build_id).wrap_err("Failed to serialize the response.")?;
		let response = http::Response::builder().body(full(body)).unwrap();
		Ok(response)
	}

	pub async fn handle_get_build_target_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "builds", id, "target"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let id = id.parse().wrap_err("Failed to parse the ID.")?;

		// Attempt to get the build target.
		let Some(build_id) = self.inner.client.try_get_build_target(&id).await? else {
			return Ok(not_found());
		};

		// Create the response.
		let body = serde_json::to_vec(&build_id).wrap_err("Failed to serialize the response.")?;
		let response = http::Response::builder().body(full(body)).unwrap();
		Ok(response)
	}

	pub async fn handle_get_build_children_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "builds", id, "children"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let id = id.parse().wrap_err("Failed to parse the ID.")?;

		// Attempt to get the children.
		let Some(children) = self.inner.client.try_get_build_children(&id).await? else {
			return Ok(not_found());
		};

		// Create the response.
		let body = Outgoing::new(StreamBody::new(
			children
				.map_ok(|id| {
					let mut id = serde_json::to_string(&id).unwrap();
					id.push('\n');
					hyper::body::Frame::data(Bytes::from(id))
				})
				.map_err(Into::into),
		));
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(body)
			.unwrap();
		Ok(response)
	}

	pub async fn handle_post_build_child_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "builds", id, "children"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let build_id: tg::build::Id = id.parse().wrap_err("Failed to parse the ID.")?;

		// Read the body.
		let bytes = request
			.into_body()
			.collect()
			.await
			.wrap_err("Failed to read the body.")?
			.to_bytes();
		let child_id =
			serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the body.")?;

		self.inner
			.client
			.add_build_child(&build_id, &child_id)
			.await?;

		// Create the response.
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(empty())
			.unwrap();
		Ok(response)
	}

	pub async fn handle_get_build_log_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "builds", id, "log"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let id = id.parse().wrap_err("Failed to parse the ID.")?;

		// Get the log.
		let Some(log) = self.inner.client.try_get_build_log(&id).await? else {
			return Ok(not_found());
		};

		// Create the response.
		let body = Outgoing::new(StreamBody::new(
			log.map_ok(hyper::body::Frame::data).map_err(Into::into),
		));
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(body)
			.unwrap();
		Ok(response)
	}

	pub async fn handle_post_build_log_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "builds", id, "log"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let build_id = id.parse().wrap_err("Failed to parse the ID.")?;

		// Read the body.
		let bytes = request
			.into_body()
			.collect()
			.await
			.wrap_err("Failed to read the body.")?
			.to_bytes();

		self.inner.client.add_build_log(&build_id, bytes).await?;

		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(empty())
			.unwrap();
		Ok(response)
	}

	pub async fn handle_get_build_result_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "builds", id, "result"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let id = id.parse().wrap_err("Failed to parse the ID.")?;

		// Attempt to get the result.
		let Some(result) = self.inner.client.try_get_build_result(&id).await? else {
			return Ok(not_found());
		};

		// Create the response.
		let result = match result {
			Ok(value) => Ok(value.data(self.inner.client.as_ref()).await?),
			Err(error) => Err(error),
		};
		let body = serde_json::to_vec(&result).wrap_err("Failed to serialize the response.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(body))
			.unwrap();
		Ok(response)
	}

	pub async fn handle_post_build_result_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "builds", build_id, "result"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let build_id = build_id.parse().wrap_err("Failed to parse the ID.")?;

		// Read the body.
		let bytes = request
			.into_body()
			.collect()
			.await
			.wrap_err("Failed to read the body.")?
			.to_bytes();
		let result = serde_json::from_slice(&bytes).wrap_err("Failed to deserialize.")?;

		// Set the build result.
		self.inner
			.client
			.set_build_result(&build_id, result)
			.await?;

		// Create the response.
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(empty())
			.unwrap();
		Ok(response)
	}

	pub async fn handle_post_build_cancel_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "builds", build_id, "cancel"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let build_id = build_id.parse().wrap_err("Failed to parse the ID.")?;

		self.inner.client.cancel_build(&build_id).await?;

		// Create the response.
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(empty())
			.unwrap();

		Ok(response)
	}

	pub async fn handle_post_build_finish_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "builds", build_id, "finish"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let build_id = build_id.parse().wrap_err("Failed to parse the ID.")?;

		self.inner.client.finish_build(&build_id).await?;

		// Create the response.
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(empty())
			.unwrap();

		Ok(response)
	}

	pub async fn handle_head_object_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "objects", id] = path_components.as_slice() else {
			return_error!("Unexpected path.")
		};
		let Ok(id) = id.parse() else {
			return Ok(bad_request());
		};

		// Get whether the object exists.
		let exists = self.inner.client.get_object_exists(&id).await?;

		// Create the response.
		let status = if exists {
			http::StatusCode::OK
		} else {
			http::StatusCode::NOT_FOUND
		};
		let response = http::Response::builder()
			.status(status)
			.body(empty())
			.unwrap();

		Ok(response)
	}

	pub async fn handle_get_object_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "objects", id] = path_components.as_slice() else {
			return_error!("Unexpected path.")
		};
		let Ok(id) = id.parse() else {
			return Ok(bad_request());
		};

		// Get the object.
		let Some(bytes) = self.inner.client.try_get_object(&id).await? else {
			return Ok(not_found());
		};

		// Create the response.
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(bytes))
			.unwrap();

		Ok(response)
	}

	pub async fn handle_put_object_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "objects", id] = path_components.as_slice() else {
			return_error!("Unexpected path.")
		};
		let Ok(id) = id.parse() else {
			return Ok(bad_request());
		};

		// Read the body.
		let bytes = request
			.into_body()
			.collect()
			.await
			.wrap_err("Failed to read the body.")?
			.to_bytes();

		// Put the object.
		let result = self.inner.client.try_put_object(&id, &bytes).await?;

		// If there are missing children, then return a bad request response.
		if let Err(missing_children) = result {
			let body = serde_json::to_vec(&missing_children)
				.wrap_err("Failed to serialize the missing children.")?;
			let response = http::Response::builder()
				.status(http::StatusCode::BAD_REQUEST)
				.body(full(body))
				.unwrap();
			return Ok(response);
		}

		// Otherwise, return an ok response.
		Ok(ok())
	}

	pub async fn handle_search_packages_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the search params.
		#[derive(serde::Deserialize, Default)]
		struct SearchParams {
			query: String,
		}
		let Some(query) = request.uri().query() else {
			return Ok(bad_request());
		};
		let search_params: SearchParams =
			serde_urlencoded::from_str(query).wrap_err("Failed to parse the search params.")?;

		// Perform the search.
		let search_results = self
			.inner
			.client
			.search_packages(&search_params.query)
			.await?;

		// Create the response.
		let body =
			serde_json::to_vec(&search_results).wrap_err("Failed to serialize the response.")?;
		let response = http::Response::builder().body(full(body)).unwrap();

		Ok(response)
	}

	pub async fn handle_get_package_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, _, "packages", name] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};

		// Get the package.
		let package = self.inner.client.get_package(name).await?;

		match package {
			Some(package) => {
				// Create the body.
				let body =
					serde_json::to_vec(&package).wrap_err("Failed to serialize the package.")?;

				// Create the response.
				let response = http::Response::builder().body(full(body)).unwrap();

				Ok(response)
			},
			None => Ok(http::Response::builder()
				.status(http::StatusCode::NOT_FOUND)
				.body(empty())
				.unwrap()),
		}
	}

	pub async fn handle_get_package_version_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, _, "packages", name, "versions", version] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};

		// Get the package.
		let source_artifact_hash = self.inner.client.get_package_version(name, version).await?;

		// Create the response.
		let response = if let Some(source_artifact_hash) = source_artifact_hash {
			http::Response::builder()
				.body(full(source_artifact_hash.to_string()))
				.unwrap()
		} else {
			not_found()
		};

		Ok(response)
	}

	pub async fn handle_publish_package_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Get the token.
		let Some(token) = get_token(&request, None) else {
			return Ok(unauthorized());
		};

		// Read the body.
		let bytes = request
			.into_body()
			.collect()
			.await
			.wrap_err("Failed to read the body.")?
			.to_bytes();
		let package_id = serde_json::from_slice(&bytes).wrap_err("Invalid request.")?;

		// Create the package.
		self.inner
			.client
			.publish_package(&token, &package_id)
			.await?;

		Ok(ok())
	}

	pub async fn handle_get_package_metadata_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "packages", id, "metadata"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};

		let Ok(package_id) = id.parse::<tg::Id>() else {
			return Ok(bad_request());
		};

		// Get the package metadata.
		let metadata = self.inner.client.get_package_metadata(&package_id).await?;

		match metadata {
			Some(metadata) => {
				// Create the body.
				let body =
					serde_json::to_vec(&metadata).wrap_err("Failed to serialize the metadata.")?;

				// Create the response.
				let response = http::Response::builder().body(full(body)).unwrap();

				Ok(response)
			},
			None => Ok(http::Response::builder()
				.status(http::StatusCode::NOT_FOUND)
				.body(empty())
				.unwrap()),
		}
	}

	pub async fn handle_get_package_dependencies_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "packages", id, "dependencies"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};

		let Ok(package_id) = id.parse::<tg::Id>() else {
			return Ok(bad_request());
		};

		// Get the package dependencies.
		let dependencies = self
			.inner
			.client
			.get_package_dependencies(&package_id)
			.await?;

		match dependencies {
			Some(dependencies) => {
				// Create the body.
				let body = serde_json::to_vec(&dependencies)
					.wrap_err("Failed to serialize the package.")?;

				// Create the response.
				let response = http::Response::builder().body(full(body)).unwrap();

				Ok(response)
			},
			None => Ok(http::Response::builder()
				.status(http::StatusCode::NOT_FOUND)
				.body(empty())
				.unwrap()),
		}
	}

	pub async fn handle_get_tracker_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "trackers", path] = path_components.as_slice() else {
			return_error!("Unexpected path.")
		};
		let path = PathBuf::from(
			urlencoding::decode(path)
				.wrap_err("Failed to decode the path.")?
				.as_ref(),
		);

		// Get the tracker.
		let Some(tracker) = self.inner.client.try_get_tracker(&path).await? else {
			return Ok(not_found());
		};

		// Create the body.
		let body = serde_json::to_vec(&tracker).wrap_err("Failed to serialize the body.")?;

		// Create the response.
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(body))
			.unwrap();

		Ok(response)
	}

	pub async fn handle_patch_tracker_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "trackers", path] = path_components.as_slice() else {
			return_error!("Unexpected path.")
		};
		let path = PathBuf::from(
			urlencoding::decode(path)
				.wrap_err("Failed to decode the path.")?
				.as_ref(),
		);

		// Read the body.
		let bytes = request
			.into_body()
			.collect()
			.await
			.wrap_err("Failed to read the body.")?
			.to_bytes();
		let tracker = serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the body.")?;

		self.inner.client.set_tracker(&path, &tracker).await?;

		Ok(ok())
	}

	pub async fn handle_create_login_request(
		&self,
		_request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Create the login.
		let login = self.inner.client.create_login().await?;

		// Create the response.
		let body = serde_json::to_string(&login).wrap_err("Failed to serialize the response.")?;
		let response = http::Response::builder()
			.status(200)
			.body(full(body))
			.unwrap();
		Ok(response)
	}

	pub async fn handle_get_login_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "logins", id] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let Ok(id) = id.parse() else {
			return Ok(bad_request());
		};

		// Get the login.
		let login = self.inner.client.get_login(&id).await?;

		// Create the response.
		let response =
			serde_json::to_string(&login).wrap_err("Failed to serialize the response.")?;
		let response = http::Response::builder()
			.status(200)
			.body(full(response))
			.unwrap();
		Ok(response)
	}

	pub async fn handle_get_current_user_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Get the token from the request.
		let Some(token) = get_token(&request, None) else {
			return Ok(unauthorized());
		};

		// Authenticate the user.
		let Some(user) = self.inner.client.get_current_user(&token).await? else {
			return Ok(unauthorized());
		};

		// Create the response.
		let body = serde_json::to_string(&user).wrap_err("Failed to serialize the user.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(body))
			.unwrap();
		Ok(response)
	}
}
