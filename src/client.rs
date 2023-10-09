pub use self::reqwest::Reqwest;
use crate::{build, object, package, target, Id, Result, Value, WrapErr};
use async_trait::async_trait;
use futures::stream::BoxStream;
use std::fmt::Debug;
use url::Url;

mod reqwest;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Login {
	pub id: Id,
	pub url: Url,
	pub token: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Package {
	pub name: String,
	pub versions: Vec<String>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SearchResult {
	pub name: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct User {
	pub id: Id,
	pub email: String,
}

/// A client.
#[async_trait]
pub trait Client: Debug + Send + Sync + 'static {
	fn clone_box(&self) -> Box<dyn Client>;

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

	async fn get_build_children(&self, id: build::Id) -> Result<BoxStream<'static, build::Id>> {
		self.try_get_build_children(id)
			.await?
			.wrap_err("Failed to get the build.")
	}

	async fn try_get_build_children(
		&self,
		id: build::Id,
	) -> Result<Option<BoxStream<'static, build::Id>>>;

	async fn get_build_log(&self, id: build::Id) -> Result<BoxStream<'static, Vec<u8>>> {
		self.try_get_build_log(id)
			.await?
			.wrap_err("Failed to get the build.")
	}

	async fn try_get_build_log(&self, id: build::Id)
		-> Result<Option<BoxStream<'static, Vec<u8>>>>;

	async fn get_build_output(&self, id: build::Id) -> Result<Option<Value>> {
		self.try_get_build_output(id)
			.await?
			.wrap_err("Failed to get the build.")
	}

	async fn try_get_build_output(&self, id: build::Id) -> Result<Option<Option<Value>>>;

	async fn clean(&self) -> Result<()>;

	async fn create_login(&self) -> Result<Login>;

	async fn get_login(&self, id: Id) -> Result<Login>;

	async fn publish_package(&self, id: package::Id) -> Result<()>;

	async fn search_packages(&self, query: &str) -> Result<Vec<SearchResult>>;

	async fn get_current_user(&self) -> Result<User>;
}
