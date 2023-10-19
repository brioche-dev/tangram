use crate::{
	artifact, build, id, object, package, target, user, user::Login, value, Artifact, Client,
	Error, Handle, Id, Package, Result, Value, Wrap, WrapErr,
};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{
	stream::{self, BoxStream},
	StreamExt, TryStreamExt,
};
use std::{
	path::{Path, PathBuf},
	sync::{Arc, Weak},
};
use tokio::io::AsyncReadExt;
use tokio_util::io::StreamReader;
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
	is_local: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct GetForPathBody {
	path: PathBuf,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SetForPathBody {
	path: PathBuf,
	id: Id,
}

impl Reqwest {
	#[must_use]
	pub fn new(url: Url, token: Option<String>) -> Self {
		let is_local = url.host().map_or(false, |host| match host {
			url::Host::Domain(domain) => domain == "localhost",
			url::Host::Ipv4(ip) => ip.is_loopback(),
			url::Host::Ipv6(ip) => ip.is_loopback(),
		});
		let client = reqwest::Client::builder()
			.http2_prior_knowledge()
			.build()
			.unwrap();
		let state = Arc::new(State {
			url,
			token: std::sync::RwLock::new(token),
			client,
			file_descriptor_semaphore: tokio::sync::Semaphore::new(16),
			is_local,
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

impl Handle for Weak<State> {
	fn upgrade(&self) -> Option<Box<dyn Client>> {
		self.upgrade()
			.map(|state| Box::new(Reqwest { state }) as Box<dyn Client>)
	}
}

#[async_trait]
impl Client for Reqwest {
	fn clone_box(&self) -> Box<dyn Client> {
		Box::new(self.clone())
	}

	fn downgrade_box(&self) -> Box<dyn Handle> {
		Box::new(Arc::downgrade(&self.state))
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
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(false);
		}
		response
			.error_for_status()
			.wrap_err("Expected the status to be success.")?;
		Ok(true)
	}

	async fn try_get_object_bytes(&self, id: object::Id) -> Result<Option<Vec<u8>>> {
		let request = self.request(http::Method::GET, &format!("/v1/objects/{id}"));
		let response = request
			.send()
			.await
			.wrap_err("Failed to send the request.")?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}
		let response = response
			.error_for_status()
			.wrap_err("Expected the status to be success.")?;
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
		if response.status() == http::StatusCode::BAD_REQUEST {
			let bytes = response
				.bytes()
				.await
				.wrap_err("Failed to get the response body.")?;
			let missing_children = tangram_serialize::from_slice(&bytes)
				.wrap_err("Failed to deserialize the body.")?;
			return Ok(Err(missing_children));
		}
		response
			.error_for_status()
			.wrap_err("Expected the status to be success.")?;
		Ok(Ok(()))
	}

	async fn try_get_build_for_target(&self, id: target::Id) -> Result<Option<build::Id>> {
		let request = self.request(http::Method::GET, &format!("/v1/targets/{id}/build"));
		let response = request
			.send()
			.await
			.wrap_err("Failed to send the request.")?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}
		let response = response
			.error_for_status()
			.wrap_err("Expected the status to be success.")?;
		let bytes = response
			.bytes()
			.await
			.wrap_err("Failed to get the response body.")?;
		let id = serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the body.")?;
		Ok(Some(id))
	}

	async fn get_or_create_build_for_target(&self, id: target::Id) -> Result<build::Id> {
		let request = self.request(http::Method::POST, &format!("/v1/targets/{id}/build"));
		let response = request
			.send()
			.await
			.wrap_err("Failed to send the request.")?
			.error_for_status()
			.wrap_err("Expected the status to be success.")?;
		let bytes = response
			.bytes()
			.await
			.wrap_err("Failed to get the response body.")?;
		let id = serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the body.")?;
		Ok(id)
	}

	async fn try_get_build_children(
		&self,
		id: build::Id,
	) -> Result<Option<BoxStream<'static, Result<build::Id>>>> {
		let request = self.request(http::Method::GET, &format!("/v1/builds/{id}/children"));
		let response = request
			.send()
			.await
			.wrap_err("Failed to send the request.")?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}
		let response = response
			.error_for_status()
			.wrap_err("Expected the status to be success.")?;
		let reader = StreamReader::new(
			response
				.bytes_stream()
				.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error)),
		);
		let children = stream::try_unfold(reader, |mut reader| async {
			let mut bytes = vec![0u8; id::SIZE];
			match reader.read_exact(&mut bytes).await {
				Ok(_) => {},
				Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
				Err(error) => return Err(error.wrap("Failed to read from the reader.")),
			};
			let id = build::Id::try_from(bytes)?;
			Ok(Some((id, reader)))
		})
		.boxed();
		Ok(Some(children))
	}

	async fn try_get_build_log(
		&self,
		id: build::Id,
	) -> Result<Option<BoxStream<'static, Result<Bytes>>>> {
		let request = self.request(http::Method::GET, &format!("/v1/builds/{id}/log"));
		let response = request
			.send()
			.await
			.wrap_err("Failed to send the request.")?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}
		let response = response
			.error_for_status()
			.wrap_err("Expected the status to be success.")?;
		let log = response
			.bytes_stream()
			.map_err(|error| error.wrap("Failed to read from the response stream."))
			.boxed();
		Ok(Some(log))
	}

	async fn try_get_build_result(&self, id: build::Id) -> Result<Option<Result<Value, Error>>> {
		let request = self.request(http::Method::GET, &format!("/v1/builds/{id}/result"));
		let response = request
			.send()
			.await
			.wrap_err("Failed to send the request.")?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}
		let response = response
			.error_for_status()
			.wrap_err("Expected the status to be success.")?;
		let bytes = response
			.bytes()
			.await
			.wrap_err("Failed to read the response body.")?;
		let result = serde_json::from_slice::<Result<value::Data, Error>>(&bytes)
			.wrap_err("Failed to deserialize the response body.")?
			.map(Value::from_data);
		Ok(Some(result))
	}

	async fn clean(&self) -> Result<()> {
		unimplemented!()
	}

	async fn create_login(&self) -> Result<Login> {
		let response = self
			.request(reqwest::Method::POST, "/v1/logins")
			.send()
			.await
			.wrap_err("Failed to send the request.")?
			.error_for_status()
			.wrap_err("Expected the status to be success.")?;
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
			.wrap_err("Expected the status to be success.")?;
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
			.wrap_err("Expected the status to be success.")?;
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
			.wrap_err("Expected the status to be success.")?;
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
			.wrap_err("Expected the status to be success.")?;
		let user = response
			.json()
			.await
			.wrap_err("Failed to get the response JSON.")?;
		Ok(user)
	}

	fn is_local(&self) -> bool {
		self.state.is_local
	}

	async fn try_get_artifact_for_path(&self, path: &Path) -> Result<Option<Artifact>> {
		let body = GetForPathBody { path: path.into() };
		let body = serde_json::to_vec(&body).wrap_err("Failed to serialize to json.")?;
		let response = self
			.request(reqwest::Method::GET, "/v1/artifact/path")
			.body(body)
			.send()
			.await
			.wrap_err("Failed to send the request.")?;
		if response.status().is_success() {
			let id: Id = response
				.json()
				.await
				.wrap_err("Failed to deserialize the response as json.")?;
			let artifact = artifact::Artifact::with_id(id.try_into()?);
			Ok(Some(artifact))
		} else if response.status().as_u16() == 404 {
			Ok(None)
		} else {
			Err(response
				.error_for_status()
				.wrap_err("The response had a non-success status.")
				.unwrap_err())
		}
	}

	async fn set_artifact_for_path(&self, path: &Path, artifact: Artifact) -> Result<()> {
		let path = path.into();
		let id = artifact.id(self).await?.into();
		let body = SetForPathBody { path, id };
		let body = serde_json::to_vec(&body).wrap_err("Failed to serialize to json.")?;
		self.request(reqwest::Method::PUT, "/v1/artifact/path")
			.body(body)
			.send()
			.await
			.wrap_err("Failed to send the request.")?
			.error_for_status()
			.wrap_err("The response had a non-success status.")?;
		Ok(())
	}

	async fn try_get_package_for_path(&self, path: &Path) -> Result<Option<Package>> {
		let body = GetForPathBody { path: path.into() };
		let body = serde_json::to_vec(&body).wrap_err("Failed to serialize to json.")?;
		let response = self
			.request(reqwest::Method::GET, "/v1/package/path")
			.body(body)
			.send()
			.await
			.wrap_err("Failed to send the request.")?;

		if response.status().is_success() {
			let id: Id = response
				.json()
				.await
				.wrap_err("Failed to deserialize the reponse as json.")?;
			let package = package::Package::with_id(id.try_into()?);
			Ok(Some(package))
		} else if response.status().as_u16() == 404 {
			Ok(None)
		} else {
			Err(response
				.error_for_status()
				.wrap_err("The response had a non-success status.")
				.unwrap_err())
		}
	}

	async fn set_package_for_path(&self, path: &Path, package: Package) -> Result<()> {
		let path = path.into();
		let id = package.id(self).await?.into();
		let body = SetForPathBody { path, id };
		let body = serde_json::to_vec(&body).wrap_err("Failed to serialize to json.")?;
		self.request(reqwest::Method::PUT, "/v1/package/path")
			.body(body)
			.send()
			.await
			.wrap_err("Failed to send the request.")?
			.error_for_status()
			.wrap_err("The response had a non-success status.")?;
		Ok(())
	}
}
