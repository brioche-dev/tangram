#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::redundant_pattern)]

pub use self::{
	artifact::Artifact,
	blob::Blob,
	build::Build,
	checksum::Checksum,
	directory::Directory,
	error::{Error, Result, WrapErr},
	file::File,
	id::Id,
	module::Module,
	package::Package,
	path::{Relpath, Subpath},
	reqwest::Reqwest,
	symlink::Symlink,
	system::System,
	target::Target,
	template::Template,
	value::Value,
};
use self::{user::Login, user::User};
use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::BoxStream;
use std::{fmt::Debug, path::Path};

pub mod artifact;
pub mod blob;
pub mod build;
pub mod bundle;
pub mod checkin;
pub mod checkout;
pub mod checksum;
pub mod directory;
pub mod error;
pub mod file;
pub mod id;
pub mod module;
pub mod object;
pub mod package;
pub mod path;
pub mod reqwest;
pub mod symlink;
pub mod system;
pub mod target;
pub mod template;
pub mod user;
pub mod util;
pub mod value;

/// A client.
#[async_trait]
pub trait Client: Debug + Send + Sync + 'static {
	fn clone_box(&self) -> Box<dyn Client>;

	fn path(&self) -> Option<&Path>;

	fn set_token(&self, token: Option<String>);

	fn file_descriptor_semaphore(&self) -> &tokio::sync::Semaphore;

	async fn get_object_exists(&self, id: object::Id) -> Result<bool>;

	async fn get_object_bytes(&self, id: object::Id) -> Result<Vec<u8>> {
		self.try_get_object_bytes(id)
			.await?
			.wrap_err("Failed to get the object.")
	}

	async fn try_get_object_bytes(&self, id: object::Id) -> Result<Option<Vec<u8>>>;

	async fn try_put_object_bytes(
		&self,
		id: object::Id,
		bytes: &[u8],
	) -> Result<Result<(), Vec<object::Id>>>;

	async fn try_get_build_for_target(&self, id: target::Id) -> Result<Option<build::Id>>;

	async fn get_or_create_build_for_target(&self, id: target::Id) -> Result<build::Id>;

	async fn get_build_children(
		&self,
		id: build::Id,
	) -> Result<BoxStream<'static, Result<build::Id>>> {
		self.try_get_build_children(id)
			.await?
			.wrap_err("Failed to get the build.")
	}

	async fn try_get_build_children(
		&self,
		id: build::Id,
	) -> Result<Option<BoxStream<'static, Result<build::Id>>>>;

	async fn get_build_log(&self, id: build::Id) -> Result<BoxStream<'static, Result<Bytes>>> {
		self.try_get_build_log(id)
			.await?
			.wrap_err("Failed to get the build.")
	}

	async fn try_get_build_log(
		&self,
		id: build::Id,
	) -> Result<Option<BoxStream<'static, Result<Bytes>>>>;

	async fn get_build_result(&self, id: build::Id) -> Result<Result<Value>> {
		self.try_get_build_result(id)
			.await?
			.wrap_err("Failed to get the build.")
	}

	async fn try_get_build_result(&self, id: build::Id) -> Result<Option<Result<Value>>>;

	async fn clean(&self) -> Result<()>;

	async fn create_login(&self) -> Result<Login>;

	async fn get_login(&self, id: Id) -> Result<Option<Login>>;

	async fn publish_package(&self, id: package::Id) -> Result<()>;

	async fn search_packages(&self, query: &str) -> Result<Vec<package::SearchResult>>;

	async fn get_current_user(&self) -> Result<User>;
}
