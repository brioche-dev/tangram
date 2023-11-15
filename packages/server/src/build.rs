use super::Server;
use crate::{ChildrenState, LogState, Progress, ProgressInner, ResultState};
use bytes::Bytes;
use futures::{
	stream::{self, BoxStream},
	StreamExt, TryStreamExt,
};
use std::sync::Arc;
use tangram_client as tg;
use tangram_error::{error, return_error, Result, Wrap, WrapErr};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_stream::wrappers::BroadcastStream;

impl Server {
	/// Attempt to get the build for a target.
	pub async fn try_get_build_for_target(
		&self,
		id: &tg::target::Id,
	) -> Result<Option<tg::build::Id>> {
		// Attempt to get the build for the target from the database.
		if let Some(build_id) = self.try_get_build_for_target_from_database(id)? {
			return Ok(Some(build_id));
		}

		// Attempt to get the build for the target from the remote.
		if let Ok(Some(build_id)) = self.try_get_build_for_target_from_remote(id).await {
			return Ok(Some(build_id));
		}

		Ok(None)
	}

	/// Attempt to get the build for the target from the database.
	fn try_get_build_for_target_from_database(
		&self,
		id: &tg::target::Id,
	) -> Result<Option<tg::build::Id>> {
		self.inner.database.try_get_build_for_target(id)
	}

	/// Attempt to get the build for the target from the remote.
	async fn try_get_build_for_target_from_remote(
		&self,
		id: &tg::target::Id,
	) -> Result<Option<tg::build::Id>> {
		// Get the remote.
		let Some(remote) = self.inner.remote.as_ref() else {
			return Ok(None);
		};

		// Get the build for the target from the remote.
		let Some(build_id) = remote.try_get_build_for_target(id).await? else {
			return Ok(None);
		};

		// Add the assignment to the database.
		self.inner.database.set_build_for_target(id, &build_id)?;

		Ok(Some(build_id))
	}

	/// Get or create a build for a target.
	pub async fn get_or_create_build_for_target(
		&self,
		id: &tg::target::Id,
	) -> Result<tg::build::Id> {
		let target = tg::Target::with_id(id.clone());

		// Attempt to get the build for the target.
		if let Some(build_id) = self.try_get_build_for_target(id).await? {
			return Ok(build_id);
		}

		// Decide whether to attempt to escalate the build.
		let escalate = true;

		// Attempt to escalate the build.
		if escalate {
			if let Some(remote) = self.inner.remote.as_ref() {
				let object = tg::object::Handle::with_id(id.clone().into());
				let result = object.push(self, remote.as_ref()).await;
				if result.is_ok() {
					if let Ok(build_id) = remote.get_or_create_build_for_target(id).await {
						return Ok(build_id);
					}
				}
			}
		}

		// Otherwise, create a new build.
		let build_id = tg::build::Id::new();

		// Create the children state.
		let children = std::sync::Mutex::new(ChildrenState {
			children: Vec::new(),
			sender: Some(tokio::sync::broadcast::channel(1024).0),
		});

		// Create the log state.
		let log = Arc::new(tokio::sync::Mutex::new(LogState {
			file: tokio::fs::File::from_std(
				tempfile::tempfile().wrap_err("Failed to create the temporary file.")?,
			),
			sender: Some(tokio::sync::broadcast::channel(1024).0),
		}));

		// Create the result state.
		let (sender, receiver) = tokio::sync::watch::channel(None);
		let result = ResultState {
			sender,
			result: receiver,
		};

		// Create the progress.
		let progress = Progress {
			inner: Arc::new(ProgressInner {
				target,
				children,
				log,
				result,
			}),
		};

		// Add the progress to the server.
		self.inner
			.progress
			.write()
			.unwrap()
			.insert(build_id.clone(), progress.clone());

		// Add the assignment to the database.
		self.inner.database.set_build_for_target(id, &build_id)?;

		// Start the build.
		self.start_build(&build_id).await?;

		Ok(build_id)
	}

	pub(super) async fn start_build(&self, id: &tg::build::Id) -> Result<()> {
		tokio::spawn({
			let server = self.clone();
			let id = id.clone();
			async move { server.start_build_inner(&id).await }
		});
		Ok(())
	}

	async fn start_build_inner(&self, id: &tg::build::Id) -> Result<()> {
		let build = tg::Build::with_id(id.clone());
		let target = build.target(self).await?;

		// Build the target with the appropriate runtime.
		let result = match target.host(self).await?.os() {
			tg::system::Os::Js => {
				// Build the target on the server's local pool because it is a `!Send` future.
				self.inner
					.local_pool
					.spawn_pinned({
						let server = self.clone();
						let build = build.clone();
						let main_runtime_handle = tokio::runtime::Handle::current();
						move || async move {
							tangram_runtime::js::build(&server, &build, main_runtime_handle).await
						}
					})
					.await
					.wrap_err("Failed to join the build task.")?
			},
			tg::system::Os::Darwin => {
				#[cfg(target_os = "macos")]
				{
					tangram_runtime::darwin::build(self, &build).await
				}
				#[cfg(not(target_os = "macos"))]
				{
					return_error!("Cannot build a darwin target on this host.");
				}
			},
			tg::system::Os::Linux => {
				#[cfg(target_os = "linux")]
				{
					tangram_runtime::linux::build(self, &build).await
				}
				#[cfg(not(target_os = "linux"))]
				{
					return_error!("Cannot build a linux target on this host.");
				}
			},
		};

		// If an error occurred, add the error to the build's log.
		if let Err(error) = result.as_ref() {
			build
				.add_log(self, error.trace().to_string().into())
				.await?;
		}

		// Finish the build.
		build.finish(self, result).await?;

		Ok(())
	}

	pub async fn get_build_from_queue(&self) -> Result<tg::build::Id> {
		let Some(remote) = self.inner.remote.as_ref() else {
			return_error!("The server does not have a remote.");
		};
		let build_id = remote.get_build_from_queue().await?;
		Ok(build_id)
	}

	pub async fn try_get_build_target(&self, id: &tg::build::Id) -> Result<Option<tg::target::Id>> {
		// Attempt to get the target from the progress.
		let progress = self.inner.progress.read().unwrap().get(id).cloned();
		if let Some(progress) = progress {
			return Ok(Some(progress.inner.target.id(self).await?.clone()));
		}

		// Attempt to get the target from the object.
		'a: {
			let build = tg::Build::with_id(id.clone());
			let Some(object) = build.try_get_object(self).await? else {
				break 'a;
			};
			return Ok(Some(object.target.id(self).await?.clone()));
		}

		// Attempt to get the target from the remote.
		'a: {
			let Some(remote) = self.inner.remote.as_ref() else {
				break 'a;
			};
			let Some(target) = remote.try_get_build_target(id).await? else {
				break 'a;
			};
			return Ok(Some(target));
		}

		Ok(None)
	}

	pub async fn try_get_build_children(
		&self,
		id: &tg::build::Id,
	) -> Result<Option<BoxStream<'static, Result<tg::build::Id>>>> {
		// Attempt to get the children from the progress.
		let progress = self.inner.progress.read().unwrap().get(id).cloned();
		if let Some(progress) = progress {
			let state = progress.inner.children.lock().unwrap();
			let old = stream::iter(state.children.clone()).map(Ok);
			let new = if let Some(sender) = state.sender.as_ref() {
				BroadcastStream::new(sender.subscribe())
					.map_err(|err| err.wrap("Failed to create the stream."))
					.boxed()
			} else {
				stream::empty().boxed()
			};
			return Ok(Some(
				old.chain(new).map_ok(|build| build.id().clone()).boxed(),
			));
		}

		// Attempt to get the children from the object.
		'a: {
			let build = tg::Build::with_id(id.clone());
			let Some(object) = build.try_get_object(self).await? else {
				break 'a;
			};
			return Ok(Some(
				stream::iter(object.children.clone())
					.map(|build| Ok(build.id().clone()))
					.boxed(),
			));
		}

		// Attempt to get the children from the remote.
		'a: {
			let Some(remote) = self.inner.remote.as_ref() else {
				break 'a;
			};
			let Some(children) = remote.try_get_build_children(id).await? else {
				break 'a;
			};
			return Ok(Some(children));
		}

		Ok(None)
	}

	pub async fn add_build_child(
		&self,
		build_id: &tg::build::Id,
		child_id: &tg::build::Id,
	) -> Result<()> {
		// Attempt to add the child to the progress.
		let progress = self.inner.progress.read().unwrap().get(build_id).cloned();
		if let Some(progress) = progress {
			let child = tg::Build::with_id(child_id.clone());
			let mut state = progress.inner.children.lock().unwrap();
			if let Some(sender) = state.sender.as_ref().cloned() {
				state.children.push(child.clone());
				sender.send(child.clone()).ok();
			}
			return Ok(());
		};

		// Attempt to add the child to the remote.
		if let Some(remote) = self.inner.remote.as_ref() {
			remote.add_build_child(build_id, child_id).await?;
			return Ok(());
		}

		return_error!("Failed to find the build.");
	}

	pub async fn try_get_build_log(
		&self,
		id: &tg::build::Id,
	) -> Result<Option<BoxStream<'static, Result<Bytes>>>> {
		// Attempt to get the log from the progress.
		let progress = self.inner.progress.read().unwrap().get(id).cloned();
		if let Some(progress) = progress {
			let mut state = progress.inner.log.lock().await;
			state
				.file
				.rewind()
				.await
				.wrap_err("Failed to rewind the log file.")?;
			let mut old = Vec::new();
			state
				.file
				.read_to_end(&mut old)
				.await
				.wrap_err("Failed to read the log.")?;
			let old = stream::once(async move { Ok(old.into()) });
			state
				.file
				.seek(std::io::SeekFrom::End(0))
				.await
				.wrap_err("Failed to seek in the log file.")?;
			let new = if let Some(sender) = state.sender.as_ref() {
				BroadcastStream::new(sender.subscribe())
					.map_err(|err| err.wrap("Failed to create the stream."))
					.boxed()
			} else {
				stream::empty().boxed()
			};
			return Ok(Some(old.chain(new).boxed()));
		}

		// Attempt to get the log from the object.
		'a: {
			let build = tg::Build::with_id(id.clone());
			let Some(object) = build.try_get_object(self).await? else {
				break 'a;
			};
			let bytes = object.log.bytes(self).await?;
			return Ok(Some(stream::once(async move { Ok(bytes.into()) }).boxed()));
		}

		// Attempt to get the log from the remote.
		'a: {
			let Some(remote) = self.inner.remote.as_ref() else {
				break 'a;
			};
			let Some(log) = remote.try_get_build_log(id).await? else {
				break 'a;
			};
			return Ok(Some(log));
		}

		Ok(None)
	}

	pub async fn add_build_log(&self, id: &tg::build::Id, bytes: Bytes) -> Result<()> {
		// Attempt to add the log to the progress.
		let progress = self.inner.progress.read().unwrap().get(id).cloned();
		if let Some(progress) = progress {
			let mut state = progress.inner.log.lock().await;
			if let Some(sender) = state.sender.as_ref().cloned() {
				state
					.file
					.seek(std::io::SeekFrom::End(0))
					.await
					.wrap_err("Failed to seek.")?;
				state
					.file
					.write_all(&bytes)
					.await
					.wrap_err("Failed to write the log.")?;
				sender.send(bytes).ok();
			}
			return Ok(());
		}

		// Attempt to add the log to the remote.
		if let Some(remote) = self.inner.remote.as_ref() {
			remote.add_build_log(id, bytes).await?;
			return Ok(());
		}

		Ok(())
	}

	pub async fn try_get_build_result(
		&self,
		id: &tg::build::Id,
	) -> Result<Option<Result<tg::Value>>> {
		// Attempt to await the result from the progress.
		let progress = self.inner.progress.read().unwrap().get(id).cloned();
		if let Some(progress) = progress {
			return Ok(Some(
				progress
					.inner
					.result
					.result
					.clone()
					.wait_for(Option::is_some)
					.await
					.unwrap()
					.clone()
					.unwrap(),
			));
		}

		// Attempt to get the result from the object.
		'a: {
			let build = tg::Build::with_id(id.clone());
			let Some(object) = build.try_get_object(self).await? else {
				break 'a;
			};
			return Ok(Some(object.result.clone()));
		}

		// Attempt to await the result from the remote.
		'a: {
			let Some(remote) = self.inner.remote.as_ref() else {
				break 'a;
			};
			let Some(result) = remote.try_get_build_result(id).await? else {
				break 'a;
			};
			return Ok(Some(result));
		}

		Ok(None)
	}

	pub async fn cancel_build(&self, id: &tg::build::Id) -> Result<()> {
		// Attempt to finish the build on the progress.
		let progress = self.inner.progress.read().unwrap().get(id).cloned();
		if let Some(progress) = progress {
			let result = Err(error!("The build was cancelled."));
			self.finish_build_with_progress(&progress, id, result)
				.await?;
			return Ok(());
		}

		// Attempt to cancel the build on the remote.
		'a: {
			let Some(remote) = self.inner.remote.as_ref() else {
				break 'a;
			};
			remote.cancel_build(id).await?;
			return Ok(());
		}

		Ok(())
	}

	pub async fn finish_build(&self, id: &tg::build::Id, result: Result<tg::Value>) -> Result<()> {
		// Attempt to finish the build on the progress.
		let progress = self.inner.progress.read().unwrap().get(id).cloned();
		if let Some(progress) = progress {
			self.finish_build_with_progress(&progress, id, result)
				.await?;
			return Ok(());
		}

		// Attempt to finish the build on the remote.
		'a: {
			let Some(remote) = self.inner.remote.as_ref() else {
				break 'a;
			};
			remote.finish_build(id, result).await?;
			return Ok(());
		}

		Ok(())
	}

	async fn finish_build_with_progress(
		&self,
		progress: &Progress,
		id: &tg::build::Id,
		result: Result<tg::Value>,
	) -> Result<()> {
		// Get the target.
		let target = progress.inner.target.clone();

		// Get the children.
		let children = {
			let mut state = progress.inner.children.lock().unwrap();
			state.sender.take();
			state.children.clone()
		};

		// Get the log.
		let log = {
			let mut state = progress.inner.log.lock().await;
			state.sender.take();
			state.file.rewind().await.wrap_err("Failed to seek.")?;
			tg::Blob::with_reader(self, &mut state.file).await?
		};

		// Create the build.
		let _build =
			tg::Build::new(self, id.clone(), target, children, log, result.clone()).await?;

		// Set the result.
		progress
			.inner
			.result
			.sender
			.send(Some(result.clone()))
			.unwrap();

		// Remove the build's progress.
		self.inner.progress.write().unwrap().remove(id);

		Ok(())
	}
}
