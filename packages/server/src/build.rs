use super::Server;
use bytes::Bytes;
use futures::{
	stream::{self, BoxStream},
	StreamExt, TryStreamExt,
};
use lmdb::Transaction;
use std::sync::Arc;
use tangram_client as tg;
use tangram_client::{build, system, target, Blob, Build, Error, Result, Target, Value, WrapErr};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_stream::wrappers::BroadcastStream;
use tokio_util::io::StreamReader;

#[derive(Clone, Debug)]
pub struct Progress {
	state: Arc<State>,
}

#[allow(clippy::type_complexity)]
#[derive(Debug)]
struct State {
	children: std::sync::Mutex<(Vec<Build>, Option<tokio::sync::broadcast::Sender<Build>>)>,
	log: Arc<
		tokio::sync::Mutex<(
			tokio::fs::File,
			Option<tokio::sync::broadcast::Sender<Vec<u8>>>,
		)>,
	>,
	output: (
		tokio::sync::watch::Sender<Option<Option<Value>>>,
		tokio::sync::watch::Receiver<Option<Option<Value>>>,
	),
}

impl Server {
	/// Attempt to get the build for a target.
	pub(crate) async fn try_get_build_for_target(
		&self,
		id: target::Id,
	) -> Result<Option<build::Id>> {
		// Attempt to get the build for the target from the running state.
		if let Some(build_id) = self.state.running.read().unwrap().0.get(&id).copied() {
			return Ok(Some(build_id));
		}

		// Attempt to get the build for the target from the database.
		if let Some(build_id) = self.try_get_build_for_target_from_database(id)? {
			return Ok(Some(build_id));
		}

		// Attempt to get the build for the target from the parent.
		if let Some(build_id) = self.try_get_build_for_target_from_parent(id).await? {
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

		// Get the assignment.
		let Some(build_id) = parent.try_get_build_for_target(id).await? else {
			return Ok(None);
		};

		Ok(Some(build_id))
	}

	/// Get or create a build for a target.
	pub(crate) async fn get_or_create_build_for_target(
		&self,
		target_id: target::Id,
	) -> Result<build::Id> {
		let target = Target::with_id(target_id);

		// Attempt to get the build for the target.
		if let Some(build_id) = self.try_get_build_for_target(target_id).await? {
			return Ok(build_id);
		}

		// Otherwise, create a new build and add it to the server's state.
		let build_id = build::Id::new();
		let progress = Progress::new()?;
		{
			let mut running = self.state.running.write().unwrap();
			running.0.insert(target_id, build_id);
			running.1.insert(build_id, progress.clone());
		}

		// Spawn the task.
		tokio::spawn({
			let server = self.clone();
			async move {
				let object = target.object(&server).await?;

				let output = match object.host.os() {
					system::Os::Js => {
						// Build the target on the server's local pool because it is a `!Send` future.
						server
							.state
							.local_pool
							.spawn_pinned({
								let server = server.clone();
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
							.wrap_err("Failed to join the task.")?
					},
					_ => todo!(),
				};

				// Set the result on the state.
				// state.set_output(output).await;

				// Create the object.
				let children = progress.children().collect().await;
				let log = StreamReader::new(
					progress
						.log()
						.await?
						.map(Bytes::from)
						.map(Ok::<_, std::io::Error>),
				);
				let log = Blob::with_reader(&server, log).await?;
				let output = progress.output().await;
				let object = build::Object {
					children,
					log,
					output,
				};

				// Store the children.
				object
					.children()
					.into_iter()
					.map(|child| {
						let server = server.clone();
						async move { child.store(&server).await }
					})
					.collect::<futures::stream::FuturesUnordered<_>>()
					.try_collect()
					.await?;

				// Get the data.
				let data = object.to_data();

				// Serialize the data.
				let bytes = data.serialize()?;

				// Store the object.
				server
					.try_put_object_bytes(build_id.into(), &bytes)
					.await
					.wrap_err("Failed to put the object.")?
					.ok()
					.wrap_err("Expected all children to be stored.")?;

				// Create a write transaction.
				let mut txn = server.state.database.env.begin_rw_txn()?;

				// Set the build for the target in the database.
				txn.put(
					server.state.database.assignments,
					&target_id.as_bytes(),
					&build_id.as_bytes(),
					lmdb::WriteFlags::empty(),
				)?;

				// Commit the transaction.
				txn.commit()?;

				// Remove the build from the running state.
				server.state.running.write().unwrap().1.remove(&build_id);

				Ok::<_, Error>(())
			}
		});

		Ok(build_id)
	}

	pub(crate) async fn try_get_build_children(
		&self,
		id: build::Id,
	) -> Result<Option<BoxStream<'static, build::Id>>> {
		let build = Build::with_id(id);

		// Attempt to stream the children from the running state.
		let state = self
			.state
			.running
			.read()
			.unwrap()
			.1
			.get(&build.id())
			.cloned();
		if let Some(state) = state {
			let children = state.children();
			return Ok(Some(children.map(|child| child.id()).boxed()));
		}

		// Attempt to get the children from the object.
		'a: {
			let Some(object) = build.try_get_object(self).await? else {
				break 'a;
			};
			return Ok(Some(
				stream::iter(object.children.clone())
					.map(|child| child.id())
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

	pub(crate) async fn try_get_build_log(
		&self,
		id: build::Id,
	) -> Result<Option<BoxStream<'static, Vec<u8>>>> {
		let build = Build::with_id(id);

		// Attempt to stream the log from the running state.
		let state = self
			.state
			.running
			.read()
			.unwrap()
			.1
			.get(&build.id())
			.cloned();
		if let Some(state) = state {
			let log = state.log().await?;
			return Ok(Some(log));
		}

		// Attempt to get the log from the object.
		'a: {
			let Some(object) = build.try_get_object(self).await? else {
				break 'a;
			};
			let object = object.clone();
			let bytes = object.log.bytes(self).await?;
			return Ok(Some(stream::once(async move { bytes }).boxed()));
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

	pub(crate) async fn try_get_build_output(
		&self,
		id: build::Id,
	) -> Result<Option<Option<Value>>> {
		let build = Build::with_id(id);

		// Attempt to await the output from the running state.
		let state = self
			.state
			.running
			.read()
			.unwrap()
			.1
			.get(&build.id())
			.cloned();
		if let Some(state) = state {
			let output = state.output().await;
			return Ok(Some(output));
		}

		// Attempt to get the output from the object.
		'a: {
			let Some(object) = build.try_get_object(self).await? else {
				break 'a;
			};
			return Ok(Some(object.output.clone()));
		}

		// Attempt to await the output from the parent.
		'a: {
			let Some(parent) = self.state.parent.as_ref() else {
				break 'a;
			};
			let Some(output) = parent.try_get_build_output(id).await? else {
				break 'a;
			};
			return Ok(Some(output));
		}

		Ok(None)
	}
}

impl Progress {
	pub fn new() -> Result<Self> {
		let (children_tx, _) = tokio::sync::broadcast::channel(1024);
		let (log_tx, _) = tokio::sync::broadcast::channel(1024);
		let (result_tx, result_rx) = tokio::sync::watch::channel(None);
		let log_file = tokio::fs::File::from_std(tempfile::tempfile()?);
		Ok(Self {
			state: Arc::new(State {
				children: std::sync::Mutex::new((Vec::new(), Some(children_tx))),
				log: Arc::new(tokio::sync::Mutex::new((log_file, Some(log_tx)))),
				output: (result_tx, result_rx),
			}),
		})
	}

	pub fn progress(&self) -> Box<dyn tangram_runtime::Progress> {
		todo!()
	}

	pub fn children(&self) -> BoxStream<'static, Build> {
		todo!()
		// let children = self.children.lock().unwrap();
		// let old = stream::iter(children.0.clone());
		// let new = if let Some(new) = children.1.as_ref() {
		// 	BroadcastStream::new(new.subscribe())
		// 		.filter_map(|result| async move { result.ok() })
		// 		.boxed()
		// } else {
		// 	stream::empty().boxed()
		// };
		// old.chain(new).boxed()
	}

	pub async fn log(&self) -> Result<BoxStream<'static, Vec<u8>>> {
		todo!()
		// let mut log = self.log.lock().await;
		// log.0.rewind().await?;
		// let mut old = Vec::new();
		// log.0.read_to_end(&mut old).await?;
		// let old = stream::once(async move { old });
		// log.0.seek(std::io::SeekFrom::End(0)).await?;
		// let new = if let Some(new) = log.1.as_ref() {
		// 	BroadcastStream::new(new.subscribe())
		// 		.filter_map(|result| async move { result.ok() })
		// 		.boxed()
		// } else {
		// 	stream::empty().boxed()
		// };
		// Ok(old.chain(new).boxed())
	}

	pub async fn output(&self) -> Option<Value> {
		todo!()
		// self.output
		// 	.1
		// 	.clone()
		// 	.wait_for(Option::is_some)
		// 	.await
		// 	.unwrap()
		// 	.clone()
		// 	.unwrap()
	}
}

impl tangram_runtime::Progress for Progress {
	fn clone_box(&self) -> Box<dyn tangram_runtime::Progress> {
		Box::new(self.clone())
	}

	fn child(&self, child: tg::Build) {
		todo!()
	}

	fn log(&self, bytes: Vec<u8>) {
		todo!()
	}

	fn output(&self, output: Option<tg::Value>) {
		todo!()
	}
}

// pub fn add_child(&self, child: Build) {
// 	let mut children = self.children.lock().unwrap();
// 	children.0.push(child.clone());
// 	children.1.as_ref().unwrap().send(child).ok();
// }

// pub fn add_log(&self, bytes: Vec<u8>) {
// 	tokio::spawn({
// 		eprintln!("{}", std::str::from_utf8(&bytes).unwrap());
// 		let log = self.log.clone();
// 		async move {
// 			let mut log = log.lock().await;
// 			log.0.seek(std::io::SeekFrom::End(0)).await.ok();
// 			log.0.write_all(&bytes).await.ok();
// 			log.1.as_ref().unwrap().send(bytes).ok();
// 		}
// 	});
// }

// pub async fn set_output(&self, output: Option<Value>) {
// 	// Set the result.
// 	self.output.0.send(Some(output)).unwrap();

// 	// End the children and log streams.
// 	self.children.lock().unwrap().1.take();
// 	self.log.lock().await.1.take();
// }
