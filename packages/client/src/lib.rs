pub use self::{
	artifact::Artifact, blob::Blob, branch::Branch, build::Build, checksum::Checksum,
	dependency::Dependency, directory::Directory, file::File, id::Id, leaf::Leaf, lock::Lock,
	mutation::Mutation, object::Object, path::Path, symlink::Symlink, system::System,
	target::Target, template::Template, tracker::Tracker, user::User, value::Value,
};
use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::BoxStream;
use tangram_error::{error, return_error, Error, Result, Wrap, WrapErr};

pub mod artifact;
pub mod blob;
pub mod branch;
pub mod build;
pub mod bundle;
pub mod checkin;
pub mod checkout;
pub mod checksum;
pub mod dependency;
pub mod directory;
pub mod file;
pub mod id;
pub mod leaf;
pub mod lock;
pub mod mutation;
pub mod object;
pub mod package;
pub mod path;
pub mod status;
pub mod symlink;
pub mod system;
pub mod target;
pub mod template;
pub mod tracker;
pub mod user;
pub mod util;
pub mod value;

#[async_trait]
pub trait Client: Send + Sync + 'static {
	fn clone_box(&self) -> Box<dyn Client>;

	fn file_descriptor_semaphore(&self) -> &tokio::sync::Semaphore;

	async fn stop(&self) -> Result<()>;

	async fn status(&self) -> Result<status::Status>;

	async fn clean(&self) -> Result<()>;

	async fn get_object_exists(&self, id: &object::Id) -> Result<bool>;

	async fn get_object(&self, id: &object::Id) -> Result<Bytes> {
		Ok(self
			.try_get_object(id)
			.await?
			.wrap_err("Failed to get the object.")?)
	}

	async fn try_get_object(&self, id: &object::Id) -> Result<Option<Bytes>>;

	async fn try_put_object(
		&self,
		id: &object::Id,
		bytes: &Bytes,
	) -> Result<Result<(), Vec<object::Id>>>;

	async fn try_get_tracker(&self, path: &std::path::Path) -> Result<Option<Tracker>>;

	async fn set_tracker(&self, path: &std::path::Path, tracker: &Tracker) -> Result<()>;

	async fn try_get_build_for_target(&self, id: &target::Id) -> Result<Option<build::Id>>;

	async fn get_or_create_build_for_target(
		&self,
		user: Option<&User>,
		id: &target::Id,
		depth: u64,
		retry: build::Retry,
	) -> Result<build::Id>;

	async fn get_build_from_queue(
		&self,
		user: Option<&User>,
		systems: Option<Vec<system::System>>,
	) -> Result<build::queue::Item>;

	async fn get_build_target(&self, id: &build::Id) -> Result<target::Id> {
		Ok(self
			.try_get_build_target(id)
			.await?
			.wrap_err("Failed to get the build.")?)
	}

	async fn try_get_build_target(&self, id: &build::Id) -> Result<Option<target::Id>>;

	async fn get_build_children(
		&self,
		id: &build::Id,
	) -> Result<BoxStream<'static, Result<build::Id>>> {
		Ok(self
			.try_get_build_children(id)
			.await?
			.wrap_err("Failed to get the build.")?)
	}

	async fn try_get_build_children(
		&self,
		id: &build::Id,
	) -> Result<Option<BoxStream<'static, Result<build::Id>>>>;

	async fn add_build_child(
		&self,
		user: Option<&User>,
		id: &build::Id,
		child_id: &build::Id,
	) -> Result<()>;

	async fn get_build_log(&self, id: &build::Id) -> Result<BoxStream<'static, Result<Bytes>>> {
		Ok(self
			.try_get_build_log(id)
			.await?
			.wrap_err("Failed to get the build.")?)
	}

	async fn try_get_build_log(
		&self,
		id: &build::Id,
	) -> Result<Option<BoxStream<'static, Result<Bytes>>>>;

	async fn add_build_log(&self, user: Option<&User>, id: &build::Id, bytes: Bytes) -> Result<()>;

	async fn get_build_outcome(&self, id: &build::Id) -> Result<build::Outcome> {
		Ok(self
			.try_get_build_outcome(id)
			.await?
			.wrap_err("Failed to get the build.")?)
	}

	async fn try_get_build_outcome(&self, id: &build::Id) -> Result<Option<build::Outcome>>;

	async fn cancel_build(&self, user: Option<&User>, id: &build::Id) -> Result<()>;

	async fn finish_build(
		&self,
		user: Option<&User>,
		id: &build::Id,
		outcome: build::Outcome,
	) -> Result<()>;

	async fn create_package_and_lock(
		&self,
		_dependency: &Dependency,
	) -> Result<(directory::Id, lock::Id)> {
		todo!()
	}

	async fn search_packages(&self, query: &str) -> Result<Vec<String>>;

	async fn get_package(&self, dependency: &Dependency) -> Result<directory::Id> {
		Ok(self
			.try_get_package(dependency)
			.await?
			.wrap_err("Failed to get the package.")?)
	}

	async fn try_get_package(&self, dependency: &Dependency) -> Result<Option<directory::Id>>;

	async fn get_package_versions(&self, dependency: &Dependency) -> Result<Vec<String>> {
		Ok(self
			.try_get_package_versions(dependency)
			.await?
			.wrap_err("Failed to get the package versions.")?)
	}

	async fn try_get_package_versions(
		&self,
		dependency: &Dependency,
	) -> Result<Option<Vec<String>>>;

	async fn get_package_metadata(&self, dependency: &Dependency) -> Result<package::Metadata> {
		Ok(self
			.try_get_package_metadata(dependency)
			.await?
			.wrap_err("Failed to get the package metadata.")?)
	}

	async fn try_get_package_metadata(
		&self,
		dependency: &Dependency,
	) -> Result<Option<package::Metadata>>;

	async fn get_package_dependencies(&self, dependency: &Dependency) -> Result<Vec<Dependency>> {
		Ok(self
			.try_get_package_dependencies(dependency)
			.await?
			.wrap_err("Failed to get the package dependencies.")?)
	}

	async fn try_get_package_dependencies(
		&self,
		dependency: &Dependency,
	) -> Result<Option<Vec<Dependency>>>;

	async fn publish_package(&self, user: Option<&User>, id: &directory::Id) -> Result<()>;

	async fn create_login(&self) -> Result<user::Login>;

	async fn get_login(&self, id: &Id) -> Result<Option<user::Login>>;

	async fn get_user_for_token(&self, token: &str) -> Result<Option<User>>;
}
