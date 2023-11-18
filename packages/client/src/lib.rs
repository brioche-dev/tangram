pub use self::{
	artifact::Artifact,
	blob::Blob,
	branch::Branch,
	build::Build,
	checksum::Checksum,
	dependency::Dependency,
	directory::Directory,
	file::File,
	id::Id,
	leaf::Leaf,
	lock::Lock,
	lockfile::Lockfile,
	mutation::Mutation,
	object::Object,
	package::{Metadata, Package},
	path::Path,
	symlink::Symlink,
	system::System,
	target::Target,
	template::Template,
	tracker::Tracker,
	value::Value,
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
pub mod lockfile;
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

/// A client.
#[async_trait]
pub trait Client: Send + Sync + 'static {
	fn clone_box(&self) -> Box<dyn Client>;

	fn is_local(&self) -> bool {
		false
	}

	fn path(&self) -> Option<&std::path::Path>;

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
		id: &target::Id,
		token: Option<String>,
	) -> Result<build::Id>;

	async fn get_build_from_queue(&self, token: Option<String>) -> Result<build::Id>;

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
		id: &build::Id,
		child_id: &build::Id,
		token: Option<String>,
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

	async fn add_build_log(
		&self,
		id: &build::Id,
		bytes: Bytes,
		token: Option<String>,
	) -> Result<()>;

	async fn get_build_result(&self, id: &build::Id) -> Result<Result<Value, Error>> {
		Ok(self
			.try_get_build_result(id)
			.await?
			.wrap_err("Failed to get the build.")?)
	}

	async fn try_get_build_result(&self, id: &build::Id) -> Result<Option<Result<Value, Error>>>;

	async fn cancel_build(&self, id: &build::Id, token: Option<String>) -> Result<()>;

	async fn finish_build(
		&self,
		id: &build::Id,
		result: Result<Value>,
		token: Option<String>,
	) -> Result<()>;

	async fn search_packages(&self, quer: &str) -> Result<Vec<package::Package>>;

	async fn get_package(&self, name: &str) -> Result<Option<package::Package>>;

	async fn get_package_version(&self, name: &str, version: &str) -> Result<Option<artifact::Id>>;

	async fn get_package_metadata(&self, id: &Id) -> Result<Option<package::Metadata>>;

	async fn get_package_dependencies(
		&self,
		id: &Id,
	) -> Result<Option<Vec<dependency::Dependency>>>;

	async fn publish_package(&self, id: &artifact::Id, token: Option<String>) -> Result<()>;

	async fn create_login(&self) -> Result<user::Login>;

	async fn get_login(&self, id: &Id) -> Result<Option<user::Login>>;

	async fn get_current_user(&self, token: Option<String>) -> Result<Option<user::User>>;
}
