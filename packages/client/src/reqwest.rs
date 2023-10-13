use crate::{
	build, login::Login, object, package, return_error, target, user, Client, Id, Result, Value,
	WrapErr,
};
use async_trait::async_trait;
use futures::stream::BoxStream;
use std::sync::Arc;
use url::Url;

#[derive(Clone, Debug)]
pub struct Reqwest {
	state: Arc<State>,
}

#[derive(Debug)]
struct State {
	client: reqwest::Client,
	file_descriptor_semaphore: tokio::sync::Semaphore,
	token: std::sync::RwLock<Option<String>>,
	url: Url,
}

impl Reqwest {
	#[must_use]
	pub fn new(url: Url, token: Option<String>) -> Self {
		let client = reqwest::Client::builder()
			.http2_prior_knowledge()
			.build()
			.unwrap();
		let state = Arc::new(State {
			url,
			token: std::sync::RwLock::new(token),
			client,
			file_descriptor_semaphore: tokio::sync::Semaphore::new(16),
		});
		Self { state }
	}

	fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
		let url = format!("{}{}", self.state.url, path.strip_prefix('/').unwrap());
		let mut request = self.state.client.request(method, url);
		if let Some(token) = self.state.token.read().unwrap().as_ref() {
			request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"));
		}
		request
	}
}

#[async_trait]
impl Client for Reqwest {
	fn clone_box(&self) -> Box<dyn Client> {
		Box::new(self.clone())
	}

	fn path(&self) -> Option<&std::path::Path> {
		None
	}

	fn set_token(&self, token: Option<String>) {
		*self.state.token.write().unwrap() = token;
	}

	fn file_descriptor_semaphore(&self) -> &tokio::sync::Semaphore {
		&self.state.file_descriptor_semaphore
	}

	async fn get_object_exists(&self, id: object::Id) -> Result<bool> {
		let request = self.request(http::Method::HEAD, &format!("/v1/objects/{id}"));
		let response = request
			.send()
			.await
			.wrap_err("Failed to send the request.")?;
		match response.status() {
			http::StatusCode::OK => Ok(true),
			http::StatusCode::NOT_FOUND => Ok(false),
			_ => return_error!(r#"Unexpected status code "{}"."#, response.status()),
		}
	}

	async fn try_get_object_bytes(&self, id: object::Id) -> Result<Option<Vec<u8>>> {
		let request = self.request(http::Method::GET, &format!("/v1/objects/{id}"));
		let response = request
			.send()
			.await
			.wrap_err("Failed to send the request.")?;
		match response.status() {
			http::StatusCode::OK => {},
			http::StatusCode::NOT_FOUND => return Ok(None),
			_ => return_error!(r#"Unexpected status code "{}"."#, response.status()),
		};
		let bytes = response
			.bytes()
			.await
			.wrap_err("Failed to get the response bytes.")?;
		Ok(Some(bytes.into()))
	}

	async fn try_put_object_bytes(
		&self,
		id: object::Id,
		bytes: &[u8],
	) -> Result<Result<(), Vec<object::Id>>> {
		let request = self
			.request(http::Method::PUT, &format!("/v1/objects/{id}"))
			.body(bytes.to_owned());
		let response = request
			.send()
			.await
			.wrap_err("Failed to send the request.")?;
		match response.status() {
			http::StatusCode::OK => Ok(Ok(())),
			http::StatusCode::BAD_REQUEST => {
				let bytes = response
					.bytes()
					.await
					.wrap_err("Failed to get the response bytes.")?;
				let missing_children = tangram_serialize::from_slice(&bytes)
					.wrap_err("Failed to deserialize the missing children.")?;
				Ok(Err(missing_children))
			},
			_ => return_error!(r#"Unexpected status code "{}"."#, response.status()),
		}
	}

	async fn try_get_build_for_target(&self, _id: target::Id) -> Result<Option<build::Id>> {
		todo!()
	}

	async fn get_or_create_build_for_target(&self, _id: target::Id) -> Result<build::Id> {
		todo!()
	}

	async fn try_get_build_children(
		&self,
		_id: build::Id,
	) -> Result<Option<BoxStream<'static, build::Id>>> {
		todo!()
	}

	async fn try_get_build_log(
		&self,
		_id: build::Id,
	) -> Result<Option<BoxStream<'static, Vec<u8>>>> {
		todo!()
	}

	async fn try_get_build_output(&self, _id: build::Id) -> Result<Option<Option<Value>>> {
		todo!()
	}

	async fn clean(&self) -> Result<()> {
		todo!()
	}

	async fn create_login(&self) -> Result<Login> {
		let response = self
			.request(reqwest::Method::POST, "/v1/logins")
			.send()
			.await
			.wrap_err("Failed to send the request.")?
			.error_for_status()
			.wrap_err("The response had a non-success status.")?;
		let response = response
			.json()
			.await
			.wrap_err("Failed to get the response JSON.")?;
		Ok(response)
	}

	async fn get_login(&self, id: Id) -> Result<Option<Login>> {
		let response = self
			.request(reqwest::Method::GET, &format!("/v1/logins/{id}"))
			.send()
			.await
			.wrap_err("Failed to send the request.")?
			.error_for_status()
			.wrap_err("The response had a non-success status.")?;
		let response = response
			.json()
			.await
			.wrap_err("Failed to get the response JSON.")?;
		Ok(response)
	}

	async fn publish_package(&self, id: package::Id) -> Result<()> {
		self.request(reqwest::Method::POST, &format!("/v1/packages/{id}"))
			.send()
			.await
			.wrap_err("Failed to send the request.")?
			.error_for_status()
			.wrap_err("The response had a non-success status.")?;
		Ok(())
	}

	async fn search_packages(&self, query: &str) -> Result<Vec<package::SearchResult>> {
		let path = &format!("/v1/packages/search?query={query}");
		let response = self
			.request(reqwest::Method::GET, path)
			.send()
			.await
			.wrap_err("Failed to send the request.")?
			.error_for_status()
			.wrap_err("The response had a non-success status.")?;
		let response = response
			.json()
			.await
			.wrap_err("Failed to get the response JSON.")?;
		Ok(response)
	}

	async fn get_current_user(&self) -> Result<user::User> {
		let response = self
			.request(reqwest::Method::GET, "/v1/user")
			.send()
			.await
			.wrap_err("Failed to send the request.")?
			.error_for_status()
			.wrap_err("The response had a non-success status.")?;
		let user = response
			.json()
			.await
			.wrap_err("Faield to get the response JSON.")?;
		Ok(user)
	}
}
