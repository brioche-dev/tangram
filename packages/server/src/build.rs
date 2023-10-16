use super::Server;
use crate::{full, not_found, Incoming, Outgoing};
use bytes::Bytes;
use futures::{
	stream::{self, BoxStream},
	StreamExt, TryStreamExt,
};
use http_body_util::StreamBody;
use lmdb::Transaction;
use std::sync::Arc;
use tangram_client as tg;
use tg::{build, system, target, Blob, Build, Result, Target, Value, WrapErr};
use tg::{return_error, Client};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_stream::wrappers::BroadcastStream;

#[derive(Clone, Debug)]
pub struct Progress {
	state: Arc<State>,
}

#[derive(Debug)]
struct State {
	id: build::Id,
	children: std::sync::Mutex<ChildrenState>,
	log: Arc<tokio::sync::Mutex<LogState>>,
	logger: std::sync::Mutex<Option<tokio::sync::mpsc::UnboundedSender<Bytes>>>,
	logger_task: std::sync::Mutex<Option<tokio::task::JoinHandle<Result<()>>>>,
	result: ResultState,
}

#[derive(Debug)]
struct ChildrenState {
	children: Vec<Build>,
	sender: Option<tokio::sync::broadcast::Sender<Build>>,
}

#[derive(Debug)]
struct LogState {
	file: tokio::fs::File,
	sender: Option<tokio::sync::broadcast::Sender<Bytes>>,
}

#[derive(Debug)]
struct ResultState {
	sender: tokio::sync::watch::Sender<Option<Result<Value>>>,
	receiver: tokio::sync::watch::Receiver<Option<Result<Value>>>,
}

impl Server {
	pub async fn handle_try_get_build_for_target_request(
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
		let Some(build_id) = self.try_get_build_for_target(id).await? else {
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
		let build_id = self.get_or_create_build_for_target(id).await?;

		// Create the response.
		let body = serde_json::to_vec(&build_id).wrap_err("Failed to serialize the response.")?;
		let response = http::Response::builder().body(full(body)).unwrap();
		Ok(response)
	}

	pub async fn handle_try_get_build_children_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "builds", id, "children"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let id = id.parse().wrap_err("Failed to parse the ID.")?;

		// Get the children.
		let Some(children) = self.try_get_build_children(id).await? else {
			return Ok(not_found());
		};

		// Create the response.
		let body = Outgoing::new(StreamBody::new(
			children
				.map_ok(|id| {
					hyper::body::Frame::data(Bytes::copy_from_slice(id.as_bytes().as_slice()))
				})
				.map_err(Into::into),
		));
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(body)
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
		let Some(log) = self.try_get_build_log(id).await? else {
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

	pub async fn handle_try_get_build_result_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "builds", id, "result"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let id = id.parse().wrap_err("Failed to parse the ID.")?;

		// Get the result.
		let Some(result) = self.try_get_build_result(id).await? else {
			return Ok(not_found());
		};

		// Create the response.
		let result = match result {
			Ok(value) => Ok(value.data(self).await?),
			Err(error) => Err(error),
		};
		let body = serde_json::to_vec(&result).wrap_err("Failed to serialize the response.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(body))
			.unwrap();
		Ok(response)
	}

	/// Attempt to get the build for a target.
	pub async fn try_get_build_for_target(&self, id: target::Id) -> Result<Option<build::Id>> {
		// Attempt to get the build for the target from the running state.
		if let Some(build_id) = self.state.running.read().unwrap().0.get(&id).copied() {
			return Ok(Some(build_id));
		}

		// Attempt to get the build for the target from the database.
		if let Some(build_id) = self.try_get_build_for_target_from_database(id)? {
			return Ok(Some(build_id));
		}

		// Attempt to get the build for the target from the parent.
		if let Ok(Some(build_id)) = self.try_get_build_for_target_from_parent(id).await {
			return Ok(Some(build_id));
		}

		Ok(None)
	}

	/// Attempt to get the build for the target from the database.
	fn try_get_build_for_target_from_database(&self, id: target::Id) -> Result<Option<build::Id>> {
		// Get the build for the target from the database.
		let txn = self.state.database.env.begin_ro_txn()?;
		match txn.get(self.state.database.assignments, &id.as_bytes()) {
			Ok(build_id) => Ok(Some(build_id.try_into().wrap_err("Invalid ID.")?)),
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => Err(error.into()),
		}
	}

	/// Attempt to get the build for the target from the parent.
	async fn try_get_build_for_target_from_parent(
		&self,
		id: target::Id,
	) -> Result<Option<build::Id>> {
		// Get the parent.
		let Some(parent) = self.state.parent.as_ref() else {
			return Ok(None);
		};

		// Get the build for the target from the parent.
		let Some(build_id) = parent.try_get_build_for_target(id).await? else {
			return Ok(None);
		};

		Ok(Some(build_id))
	}

	/// Get or create a build for a target.
	pub async fn get_or_create_build_for_target(&self, target_id: target::Id) -> Result<build::Id> {
		// Attempt to get the build for the target.
		if let Some(build_id) = self.try_get_build_for_target(target_id).await? {
			return Ok(build_id);
		}

		// Otherwise, create a new build and add its progress to the server.
		let build_id = build::Id::new();
		let progress = Progress::new(build_id)?;
		{
			let mut running = self.state.running.write().unwrap();
			running.0.insert(target_id, build_id);
			running.1.insert(build_id, progress.clone());
		}

		// Spawn the task.
		tokio::spawn({
			let server = self.clone();
			async move { server.build_inner(target_id, build_id, progress).await }
		});

		Ok(build_id)
	}

	async fn build_inner(
		&self,
		target_id: target::Id,
		build_id: build::Id,
		progress: Progress,
	) -> Result<()> {
		// Build the target.
		let target = Target::with_id(target_id);
		match target.host(self).await?.os() {
			system::Os::Js => {
				// Build the target on the server's local pool because it is a `!Send` future.
				self.state
					.local_pool
					.spawn_pinned({
						let server = self.clone();
						let target = target.clone();
						let progress = progress.clone();
						let main_runtime_handle = tokio::runtime::Handle::current();
						move || async move {
							tangram_runtime::js::run(
								&server,
								target,
								&progress,
								main_runtime_handle,
							)
							.await
						}
					})
					.await
					.wrap_err("Failed to join the task.")?;
			},
			system::Os::Darwin => {
				#[cfg(target_os = "macos")]
				tangram_runtime::darwin::run(self, target, &progress).await;
				#[cfg(not(target_os = "macos"))]
				return_error!("Cannot run a darwin build on this host.");
			},
			system::Os::Linux => {
				#[cfg(target_os = "linux")]
				tangram_runtime::linux::run(self, target, &progress).await;
				#[cfg(not(target_os = "linux"))]
				return_error!("Cannot run a linux build on this host.");
			},
		};

		// Finish the build.
		let _build = progress.finish(self).await?;

		// Create a write transaction.
		let mut txn = self.state.database.env.begin_rw_txn()?;

		// Set the build for the target.
		txn.put(
			self.state.database.assignments,
			&target_id.as_bytes(),
			&build_id.as_bytes(),
			lmdb::WriteFlags::empty(),
		)?;

		// Commit the transaction.
		txn.commit()?;

		// Remove the state of the running build.
		self.state.running.write().unwrap().1.remove(&build_id);

		Ok(())
	}

	pub async fn try_get_build_children(
		&self,
		id: build::Id,
	) -> Result<Option<BoxStream<'static, Result<build::Id>>>> {
		// Attempt to stream the children from the running state.
		let state = self.state.running.read().unwrap().1.get(&id).cloned();
		if let Some(state) = state {
			let children = state.children_stream();
			return Ok(Some(children.map_ok(|child| child.id()).boxed()));
		}

		// Attempt to get the children from the object.
		'a: {
			let build = Build::with_id(id);
			let Some(object) = build.try_get_object(self).await? else {
				break 'a;
			};
			return Ok(Some(
				stream::iter(object.children.clone())
					.map(|child| child.id())
					.map(Ok)
					.boxed(),
			));
		}

		// Attempt to stream the children from the parent.
		'a: {
			let Some(parent) = self.state.parent.as_ref() else {
				break 'a;
			};
			let Some(children) = parent.try_get_build_children(id).await? else {
				break 'a;
			};
			return Ok(Some(children));
		}

		Ok(None)
	}

	pub async fn try_get_build_log(
		&self,
		id: build::Id,
	) -> Result<Option<BoxStream<'static, Result<Bytes>>>> {
		// Attempt to stream the log from the running state.
		let state = self.state.running.read().unwrap().1.get(&id).cloned();
		if let Some(state) = state {
			let log = state.log_stream().await?;
			return Ok(Some(log));
		}

		// Attempt to get the log from the object.
		'a: {
			let build = Build::with_id(id);
			let Some(object) = build.try_get_object(self).await? else {
				break 'a;
			};
			let bytes = object.log.bytes(self).await?;
			return Ok(Some(stream::once(async move { Ok(bytes.into()) }).boxed()));
		}

		// Attempt to stream the log from the parent.
		'a: {
			let Some(parent) = self.state.parent.as_ref() else {
				break 'a;
			};
			let Some(log) = parent.try_get_build_log(id).await? else {
				break 'a;
			};
			return Ok(Some(log));
		}

		Ok(None)
	}

	pub async fn try_get_build_result(&self, id: build::Id) -> Result<Option<Result<Value>>> {
		// Attempt to await the result from the running state.
		let state = self.state.running.read().unwrap().1.get(&id).cloned();
		if let Some(state) = state {
			let result = state.wait_for_result().await;
			return Ok(Some(result));
		}

		// Attempt to get the result from the object.
		'a: {
			let build = Build::with_id(id);
			let Some(object) = build.try_get_object(self).await? else {
				break 'a;
			};
			return Ok(Some(object.result.clone()));
		}

		// Attempt to await the result from the parent.
		'a: {
			let Some(parent) = self.state.parent.as_ref() else {
				break 'a;
			};
			let Some(result) = parent.try_get_build_result(id).await? else {
				break 'a;
			};
			return Ok(Some(result));
		}

		Ok(None)
	}
}

impl Progress {
	pub fn new(id: build::Id) -> Result<Self> {
		// Create the children state.
		let children = std::sync::Mutex::new(ChildrenState {
			children: Vec::new(),
			sender: Some(tokio::sync::broadcast::channel(1024).0),
		});

		// Create the log state.
		let log = Arc::new(tokio::sync::Mutex::new(LogState {
			file: tokio::fs::File::from_std(tempfile::tempfile()?),
			sender: Some(tokio::sync::broadcast::channel(1024).0),
		}));

		// Spawn the logger task.
		let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<Bytes>();
		let logger = std::sync::Mutex::new(Some(sender));
		let logger_task = std::sync::Mutex::new(Some(tokio::spawn({
			let log = log.clone();
			async move {
				while let Some(bytes) = receiver.recv().await {
					let mut log = log.lock().await;
					log.file.seek(std::io::SeekFrom::End(0)).await?;
					log.file.write_all(&bytes).await?;
					log.sender.as_ref().unwrap().send(bytes).ok();
				}
				Ok(())
			}
		})));

		// Create the result state.
		let (sender, receiver) = tokio::sync::watch::channel(None);
		let result = ResultState { sender, receiver };

		Ok(Self {
			state: Arc::new(State {
				id,
				children,
				log,
				logger,
				logger_task,
				result,
			}),
		})
	}

	pub fn children_stream(&self) -> BoxStream<'static, Result<Build>> {
		let state = self.state.children.lock().unwrap();
		let old = stream::iter(state.children.clone()).map(Ok);
		let new = if let Some(sender) = state.sender.as_ref() {
			BroadcastStream::new(sender.subscribe())
				.map_err(Into::into)
				.boxed()
		} else {
			stream::empty().boxed()
		};
		old.chain(new).boxed()
	}

	pub async fn log_stream(&self) -> Result<BoxStream<'static, Result<Bytes>>> {
		let mut log = self.state.log.lock().await;
		log.file.rewind().await?;
		let mut old = Vec::new();
		log.file.read_to_end(&mut old).await?;
		let old = stream::once(async move { Ok(old.into()) });
		log.file.seek(std::io::SeekFrom::End(0)).await?;
		let new = if let Some(sender) = log.sender.as_ref() {
			BroadcastStream::new(sender.subscribe())
				.map_err(Into::into)
				.boxed()
		} else {
			stream::empty().boxed()
		};
		Ok(old.chain(new).boxed())
	}

	pub async fn wait_for_result(&self) -> Result<Value> {
		self.state
			.result
			.receiver
			.clone()
			.wait_for(Option::is_some)
			.await
			.unwrap()
			.clone()
			.unwrap()
	}

	pub async fn finish(self, client: &dyn Client) -> Result<Build> {
		// Drop the children sender.
		self.state.children.lock().unwrap().sender.take();

		// Drop the logger sender and wait for the logger task to finish.
		self.state.logger.lock().unwrap().take();
		let logger_task = self.state.logger_task.lock().unwrap().take().unwrap();
		logger_task.await.unwrap()?;

		// Get the children.
		let children = self.state.children.lock().unwrap().children.clone();

		// Get the log.
		let log = {
			let mut state = self.state.log.lock().await;
			state.file.rewind().await?;
			Blob::with_reader(client, &mut state.file).await?
		};

		// Get the result.
		let result = self.state.result.receiver.borrow().clone().unwrap();

		// Create the build.
		let build = Build::new(client, self.state.id, children, log, result).await?;

		Ok(build)
	}
}

impl tangram_runtime::Progress for Progress {
	fn clone_box(&self) -> Box<dyn tangram_runtime::Progress> {
		Box::new(self.clone())
	}

	fn child(&self, child: &tg::Build) {
		let mut state = self.state.children.lock().unwrap();
		state.children.push(child.clone());
		state.sender.as_ref().unwrap().send(child.clone()).ok();
	}

	fn log(&self, bytes: Bytes) {
		eprintln!("{}", std::str::from_utf8(&bytes).unwrap());
		self.state
			.logger
			.lock()
			.unwrap()
			.as_ref()
			.unwrap()
			.send(bytes)
			.unwrap();
	}

	fn result(&self, result: Result<tg::Value>) {
		self.state.result.sender.send(Some(result)).unwrap();
	}
}
