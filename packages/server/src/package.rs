use crate::Server;
use tangram_client as tg;
use tangram_error::{Result, WrapErr};
use tangram_package::Ext;
use tg::Handle;

impl Server {
	pub async fn create_package_and_lock(
		&self,
		dependency: &tg::Dependency,
	) -> Result<(tg::directory::Id, tg::lock::Id)> {
		let (package, lock) = tangram_package::new(self, dependency).await?;
		Ok((
			package.id(self).await?.clone(),
			lock.id(self).await?.clone(),
		))
	}

	pub async fn search_packages(&self, query: &str) -> Result<Vec<String>> {
		self.inner
			.remote
			.as_ref()
			.wrap_err("The server does not have a remote.")?
			.search_packages(query)
			.await
	}

	pub async fn try_get_package(
		&self,
		dependency: &tg::Dependency,
	) -> Result<Option<tg::directory::Id>> {
		if let Some(id) = dependency.id.as_ref() {
			return Ok(Some(id.clone()));
		}

		self.inner
			.remote
			.as_ref()
			.wrap_err("The server does not have a remote.")?
			.try_get_package(dependency)
			.await
	}

	pub async fn try_get_package_versions(
		&self,
		dependency: &tg::Dependency,
	) -> Result<Option<Vec<String>>> {
		self.inner
			.remote
			.as_ref()
			.wrap_err("The server does not have a remote.")?
			.try_get_package_versions(dependency)
			.await
	}

	pub async fn try_get_package_metadata(
		&self,
		dependency: &tg::Dependency,
	) -> Result<Option<tg::package::Metadata>> {
		let package = tg::Directory::with_id(self.get_package(dependency).await?);
		let metadata = package.metadata(self).await?;
		Ok(Some(metadata))
	}

	pub async fn try_get_package_dependencies(
		&self,
		dependency: &tg::Dependency,
	) -> Result<Option<Vec<tg::Dependency>>> {
		let package = tg::Directory::with_id(self.get_package(dependency).await?);
		let dependencies = package.dependencies(self).await?;
		Ok(Some(dependencies))
	}

	pub async fn publish_package(
		&self,
		user: Option<&tg::User>,
		id: &tg::directory::Id,
	) -> Result<()> {
		let remote = self
			.inner
			.remote
			.as_ref()
			.wrap_err("The server does not have a remote.")?;
		tg::object::Handle::with_id(id.clone().into())
			.push(self, remote.as_ref())
			.await
			.wrap_err("Failed to push the package.")?;
		remote.publish_package(user, id).await
	}
}
