use super::Server;
use crate::Progress;
use bytes::Bytes;
use futures::{
	stream::{self, BoxStream},
	StreamExt, TryStreamExt,
};
use lmdb::Transaction;
use tangram_client as tg;
use tg::{return_error, Result, Wrap, WrapErr};

impl Server {
	/// Attempt to get the build for a target.
	pub async fn try_get_build_for_target(
		&self,
		id: &tg::target::Id,
	) -> Result<Option<tg::build::Id>> {
		// Attempt to get the build for the target from the state.
		if let Some(build_id) = self
			.inner
			.state
			.assignments
			.read()
			.unwrap()
			.get(id)
			.cloned()
		{
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
	fn try_get_build_for_target_from_database(
		&self,
		id: &tg::target::Id,
	) -> Result<Option<tg::build::Id>> {
		// Get the build for the target from the database.
		let txn = self
			.inner
			.database
			.env
			.begin_ro_txn()
			.wrap_err("Failed to begin the transaction.")?;
		let build_id = match txn.get(self.inner.database.assignments, &id.to_bytes()) {
			Ok(build_id) => build_id,
			Err(lmdb::Error::NotFound) => return Ok(None),
			Err(error) => return Err(error.wrap("Failed to get the assignment.")),
		};
		let build_id = build_id.try_into().wrap_err("Invalid ID.")?;
		Ok(Some(build_id))
	}

	/// Attempt to get the build for the target from the parent.
	async fn try_get_build_for_target_from_parent(
		&self,
		id: &tg::target::Id,
	) -> Result<Option<tg::build::Id>> {
		// Get the parent.
		let Some(parent) = self.inner.parent.as_ref() else {
			return Ok(None);
		};

		// Get the build for the target from the parent.
		let Some(build_id) = parent.try_get_build_for_target(id).await? else {
			return Ok(None);
		};

		Ok(Some(build_id))
	}

	/// Get or create a build for a target.
	pub async fn get_or_create_build_for_target(
		&self,
		target_id: &tg::target::Id,
	) -> Result<tg::build::Id> {
		let target = tg::Target::with_id(target_id.clone());

		// Attempt to get the build for the target.
		if let Some(build_id) = self.try_get_build_for_target(target_id).await? {
			return Ok(build_id);
		}

		// Otherwise, create a new build and add its progress to the server's state.
		let build_id = tg::build::Id::new();
		let progress = Progress::new(build_id.clone(), target)?;
		self.inner
			.state
			.progress
			.write()
			.unwrap()
			.insert(build_id.clone(), progress.clone());
		self.inner
			.state
			.assignments
			.write()
			.unwrap()
			.insert(target_id.clone(), build_id.clone());

		// Spawn the task.
		tokio::spawn({
			let server = self.clone();
			let target_id = target_id.clone();
			let build_id = build_id.clone();
			async move { server.build_inner(target_id, build_id).await }
		});

		Ok(build_id)
	}

	async fn build_inner(&self, target_id: tg::target::Id, build_id: tg::build::Id) -> Result<()> {
		let build = tg::Build::with_id(build_id.clone());
		let target = tg::Target::with_id(target_id.clone());

		// Build.
		match target.host(self).await?.os() {
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
					.wrap_err("Failed to run the build.")?;
			},
			tg::system::Os::Darwin => {
				#[cfg(target_os = "macos")]
				tangram_runtime::darwin::build(self, &build).await?;
				#[cfg(not(target_os = "macos"))]
				return_error!("Cannot build a darwin target on this host.");
			},
			tg::system::Os::Linux => {
				#[cfg(target_os = "linux")]
				tangram_runtime::linux::build(self, &build).await?;
				#[cfg(not(target_os = "linux"))]
				return_error!("Cannot build a linux target on this host.");
			},
		};

		// Finish the build.
		build.finish(self).await?;

		Ok(())
	}

	pub async fn try_get_build_target(&self, id: &tg::build::Id) -> Result<Option<tg::target::Id>> {
		// Attempt to get the target from the state.
		let progress = self.inner.state.progress.read().unwrap().get(id).cloned();
		if let Some(progress) = progress {
			return Ok(Some(progress.target().id(self).await?.clone()));
		}

		// Attempt to get the target from the object.
		'a: {
			let build = tg::Build::with_id(id.clone());
			let Some(object) = build.try_get_object(self).await? else {
				break 'a;
			};
			return Ok(Some(object.target.id(self).await?.clone()));
		}

		Ok(None)
	}

	pub async fn try_get_build_children(
		&self,
		id: &tg::build::Id,
	) -> Result<Option<BoxStream<'static, Result<tg::build::Id>>>> {
		// Attempt to stream the children from the state.
		let progress = self.inner.state.progress.read().unwrap().get(id).cloned();
		if let Some(progress) = progress {
			return Ok(Some(
				progress
					.children()
					.and_then({
						let server = self.clone();
						move |child| {
							let server = server.clone();
							async move { Ok(child.id(&server).await?.clone()) }
						}
					})
					.boxed(),
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
					.map(Ok)
					.and_then({
						let server = self.clone();
						move |child| {
							let server = server.clone();
							async move { Ok(child.id(&server).await?.clone()) }
						}
					})
					.boxed(),
			));
		}

		// Attempt to stream the children from the parent.
		'a: {
			let Some(parent) = self.inner.parent.as_ref() else {
				break 'a;
			};
			let Some(children) = parent.try_get_build_children(id).await? else {
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
		// Get the progress for the build.
		let progress = self
			.inner
			.state
			.progress
			.read()
			.unwrap()
			.get(build_id)
			.cloned()
			.wrap_err("Failed to find the build.")?;

		// Add the child.
		progress.add_child(&tg::Build::with_id(child_id.clone()));

		Ok(())
	}

	pub async fn try_get_build_log(
		&self,
		id: &tg::build::Id,
	) -> Result<Option<BoxStream<'static, Result<Bytes>>>> {
		// Attempt to stream the log from the state.
		let progress = self.inner.state.progress.read().unwrap().get(id).cloned();
		if let Some(progress) = progress {
			return Ok(Some(progress.log().await?));
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

		// Attempt to stream the log from the parent.
		'a: {
			let Some(parent) = self.inner.parent.as_ref() else {
				break 'a;
			};
			let Some(log) = parent.try_get_build_log(id).await? else {
				break 'a;
			};
			return Ok(Some(log));
		}

		Ok(None)
	}

	pub async fn add_build_log(&self, build_id: &tg::build::Id, log: Bytes) -> Result<()> {
		// Get the progress for the build.
		let progress = self
			.inner
			.state
			.progress
			.read()
			.unwrap()
			.get(build_id)
			.cloned()
			.wrap_err("Failed to find the build.")?;

		// Add the log.
		progress.add_log(log).await?;

		Ok(())
	}

	pub async fn try_get_build_result(
		&self,
		id: &tg::build::Id,
	) -> Result<Option<Result<tg::Value>>> {
		// Attempt to await the result from the state.
		let progress = self.inner.state.progress.read().unwrap().get(id).cloned();
		if let Some(progress) = progress {
			return Ok(Some(progress.result().await));
		}

		// Attempt to get the result from the object.
		'a: {
			let build = tg::Build::with_id(id.clone());
			let Some(object) = build.try_get_object(self).await? else {
				break 'a;
			};
			return Ok(Some(object.result.clone()));
		}

		// Attempt to await the result from the parent.
		'a: {
			let Some(parent) = self.inner.parent.as_ref() else {
				break 'a;
			};
			let Some(result) = parent.try_get_build_result(id).await? else {
				break 'a;
			};
			return Ok(Some(result));
		}

		Ok(None)
	}

	pub async fn set_build_result(
		&self,
		build_id: &tg::build::Id,
		result: Result<tg::Value>,
	) -> Result<()> {
		// Get the progress for the build.
		let progress = self
			.inner
			.state
			.progress
			.read()
			.unwrap()
			.get(build_id)
			.cloned()
			.wrap_err("Failed to find the build.")?;

		// Set the result.
		progress.set_result(result);

		Ok(())
	}

	pub async fn finish_build(&self, build_id: &tg::build::Id) -> Result<()> {
		let build = tg::Build::with_id(build_id.clone());
		let target = build.target(self).await?;
		let target_id = target.id(self).await?;

		// Finish the build.
		let progress = self
			.inner
			.state
			.progress
			.read()
			.unwrap()
			.get(build_id)
			.cloned();
		progress
			.wrap_err("Failed to find the build.")?
			.finish(self)
			.await?;

		// Create a write transaction.
		let mut txn = self
			.inner
			.database
			.env
			.begin_rw_txn()
			.wrap_err("Failed to begin the transaction.")?;

		// Set the build for the target.
		txn.put(
			self.inner.database.assignments,
			&target_id.to_bytes(),
			&build_id.to_bytes(),
			lmdb::WriteFlags::empty(),
		)
		.wrap_err("Failed to store the item.")?;

		// Commit the transaction.
		txn.commit().wrap_err("Failed to commit the transaction.")?;

		// Remove the build from the server's state.
		self.inner
			.state
			.assignments
			.write()
			.unwrap()
			.remove(target_id);
		self.inner.state.progress.write().unwrap().remove(build_id);

		Ok(())
	}
}
