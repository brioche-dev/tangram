use crate::{
	artifact, build, object, package, target, user, user::Login, Artifact, Client, Handle, Id,
	Package, Result, Value, Wrap, WrapErr,
};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{stream::BoxStream, StreamExt, TryStreamExt};
use http_body_util::{BodyExt, BodyStream};
use std::{
	path::{Path, PathBuf},
	sync::{Arc, RwLock, Weak},
};
use tangram_error::return_error;
use tangram_util::{addr::Addr, empty, full, Incoming, Outgoing};
use tokio::{
	io::AsyncBufReadExt,
	net::{TcpStream, UnixStream},
};
use tokio_stream::wrappers::LinesStream;
use tokio_util::io::StreamReader;

#[derive(Debug, Clone)]
pub struct Remote {
	state: Arc<State>,
}

#[derive(Debug)]
struct State {
	addr: tangram_util::addr::Addr,
	file_descriptor_semaphore: tokio::sync::Semaphore,
	sender: hyper::client::conn::http2::SendRequest<Outgoing>,
	token: std::sync::RwLock<Option<String>>,
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

impl Handle for Weak<State> {
	fn upgrade(&self) -> Option<Box<dyn Client>> {
		self.upgrade()
			.map(|state| Box::new(Remote { state }) as Box<dyn Client>)
	}
}

impl Remote {
	pub async fn new(addr: Addr, token: Option<String>) -> Result<Self> {
		let file_descriptor_semaphore = tokio::sync::Semaphore::new(16);
		let sender = match &addr {
			Addr::Inet(url) if url.scheme() == "https" => {
				let addr = format!("{}:{}", url.host_str().unwrap(), url.port().unwrap());
				let stream = TcpStream::connect(addr)
					.await
					.wrap_err("Failed to create the TCP connection.")?;
				let mut root_cert_store = rustls::RootCertStore::empty();
				root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
				let config = rustls::ClientConfig::builder()
					.with_safe_defaults()
					.with_root_certificates(root_cert_store)
					.with_no_client_auth();
				let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
				let domain = url.domain().wrap_err("Missing domain for URL.")?;
				let domain = rustls::ServerName::try_from(domain)
					.wrap_err("Failed to create the server name.")?;
				let stream = connector
					.connect(domain, stream)
					.await
					.wrap_err("Failed to connect.")?;
				let executor = hyper_util::rt::TokioExecutor::new();
				let io = hyper_util::rt::TokioIo::new(stream);
				let (sender, connection) = hyper::client::conn::http2::handshake(executor, io)
					.await
					.wrap_err("Failed to perform the HTTP handshake.")?;
				tokio::spawn(connection);
				sender
			},
			Addr::Inet(url) => {
				let addr = format!("{}:{}", url.host_str().unwrap(), url.port().unwrap());
				let stream = TcpStream::connect(addr)
					.await
					.wrap_err("Failed to create the TCP connection.")?;
				let executor = hyper_util::rt::TokioExecutor::new();
				let io = hyper_util::rt::TokioIo::new(stream);
				let (sender, connection) = hyper::client::conn::http2::handshake(executor, io)
					.await
					.wrap_err("Failed to perform the HTTP handshake.")?;
				tokio::spawn(connection);
				sender
			},
			Addr::Unix(path) => {
				let stream = UnixStream::connect(path)
					.await
					.wrap_err("Failed to connect to the socket.")?;
				let executor = hyper_util::rt::TokioExecutor::new();
				let io = hyper_util::rt::TokioIo::new(stream);
				let (sender, connection) = hyper::client::conn::http2::handshake(executor, io)
					.await
					.wrap_err("Failed to perform the HTTP handshake.")?;
				tokio::spawn(connection);
				sender
			},
		};
		let token = RwLock::new(token);
		let state = Arc::new(State {
			addr,
			file_descriptor_semaphore,
			sender,
			token,
		});
		Ok(Self { state })
	}

	fn request(&self, method: http::Method, path: &str) -> http::request::Builder {
		let uri = match &self.state.addr {
			Addr::Inet(url) => format!("{}{}", url, path.strip_prefix('/').unwrap()),
			Addr::Unix(_) => path.into(),
		};
		http::request::Builder::default().uri(uri).method(method)
	}

	async fn send(
		&self,
		request: http::request::Request<Outgoing>,
	) -> Result<http::Response<Incoming>> {
		self.state
			.sender
			.clone()
			.send_request(request)
			.await
			.wrap_err("Failed to send the request.")
	}

	pub async fn ping(&self) -> Result<()> {
		let request = self
			.request(http::Method::GET, "/v1/ping")
			.body(empty())
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		Ok(())
	}

	pub async fn stop(&self) -> Result<()> {
		let request = self
			.request(http::Method::PUT, "/v1/stop")
			.body(empty())
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		Ok(())
	}
}

#[async_trait]
impl Client for Remote {
	fn clone_box(&self) -> Box<dyn Client> {
		Box::new(self.clone())
	}

	fn downgrade_box(&self) -> Box<dyn Handle> {
		Box::new(Arc::downgrade(&self.state))
	}

	fn is_local(&self) -> bool {
		self.state.addr.is_local()
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

	async fn get_object_exists(&self, id: &object::Id) -> Result<bool> {
		let request = self
			.request(http::Method::HEAD, &format!("/v1/objects/{id}"))
			.body(empty())
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(false);
		}
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		Ok(true)
	}

	async fn try_get_object_bytes(&self, id: &object::Id) -> Result<Option<Bytes>> {
		let request = self
			.request(http::Method::GET, &format!("/v1/objects/{id}"))
			.body(empty())
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		let bytes = response
			.collect()
			.await
			.wrap_err("Failed to collect the response body.")?
			.to_bytes();
		Ok(Some(bytes))
	}

	async fn try_put_object_bytes(
		&self,
		id: &object::Id,
		bytes: &Bytes,
	) -> Result<Result<(), Vec<object::Id>>> {
		let body = full(bytes.clone());
		let request = self
			.request(http::Method::PUT, &format!("/v1/objects/{id}"))
			.body(body)
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if response.status() == http::StatusCode::BAD_REQUEST {
			let bytes = response
				.collect()
				.await
				.wrap_err("Failed to collect the response body.")?
				.to_bytes();
			let missing_children =
				serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the body.")?;
			return Ok(Err(missing_children));
		}
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		Ok(Ok(()))
	}

	async fn try_get_artifact_for_path(&self, path: &Path) -> Result<Option<Artifact>> {
		let body = GetForPathBody { path: path.into() };
		let body = serde_json::to_vec(&body).wrap_err("Failed to serialize to json.")?;
		let request = self
			.request(http::Method::GET, "/v1/artifact/path")
			.body(full(body))
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		let bytes = response
			.collect()
			.await
			.wrap_err("Failed to collect the response body.")?
			.to_bytes();
		let id: artifact::Id =
			serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the body.")?;
		let artifact = Artifact::with_id(id);
		Ok(Some(artifact))
	}

	async fn set_artifact_for_path(&self, path: &Path, artifact: &Artifact) -> Result<()> {
		let path = path.into();
		let id = artifact.id(self).await?.into();
		let body = SetForPathBody { path, id };
		let body = serde_json::to_vec(&body).wrap_err("Failed to serialize to json.")?;
		let request = self
			.request(reqwest::Method::PUT, "/v1/artifact/path")
			.body(full(body))
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		Ok(())
	}

	async fn try_get_package_for_path(&self, path: &Path) -> Result<Option<Package>> {
		let body = GetForPathBody { path: path.into() };
		let body = serde_json::to_vec(&body).wrap_err("Failed to serialize to json.")?;
		let request = self
			.request(reqwest::Method::GET, "/v1/package/path")
			.body(full(body))
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		let bytes = response
			.collect()
			.await
			.wrap_err("Failed to collect the response body.")?
			.to_bytes();
		let id: package::Id =
			serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the body.")?;
		let package = Package::with_id(id);
		Ok(Some(package))
	}

	async fn set_package_for_path(&self, path: &Path, package: &Package) -> Result<()> {
		let path = path.into();
		let id = package.id(self).await?.clone().into();
		let body = SetForPathBody { path, id };
		let body = serde_json::to_vec(&body).wrap_err("Failed to serialize to json.")?;
		let request = self
			.request(reqwest::Method::PUT, "/v1/package/path")
			.body(full(body))
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		Ok(())
	}

	async fn try_get_build_for_target(&self, id: &target::Id) -> Result<Option<build::Id>> {
		let request = self
			.request(http::Method::GET, &format!("/v1/targets/{id}/build"))
			.body(empty())
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		let bytes = response
			.collect()
			.await
			.wrap_err("Failed to collect the response body.")?
			.to_bytes();
		let id = serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the body.")?;
		Ok(Some(id))
	}

	async fn get_or_create_build_for_target(&self, id: &target::Id) -> Result<build::Id> {
		let request = self
			.request(http::Method::POST, &format!("/v1/targets/{id}/build"))
			.body(empty())
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		let bytes = response
			.collect()
			.await
			.wrap_err("Failed to collect the response body.")?
			.to_bytes();
		let id = serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the body.")?;
		Ok(id)
	}

	async fn try_get_build_children(
		&self,
		id: &build::Id,
	) -> Result<Option<BoxStream<'static, Result<build::Id>>>> {
		let request = self
			.request(http::Method::GET, &format!("/v1/builds/{id}/children"))
			.body(empty())
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		let stream = BodyStream::new(response.into_body())
			.filter_map(|frame| async {
				match frame.map(http_body::Frame::into_data) {
					Ok(Ok(bytes)) => Some(Ok(bytes)),
					Err(e) => Some(Err(e)),
					Ok(Err(_frame)) => None,
				}
			})
			.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error));
		let reader = tokio::io::BufReader::new(StreamReader::new(stream));
		let children = LinesStream::new(reader.lines())
			.map_err(|error| error.wrap("Failed to read from the reader."))
			.map(|line| {
				let line = line?;
				let id = serde_json::from_str(&line).wrap_err("Failed to deserialize the ID.")?;
				Ok(id)
			})
			.boxed();
		Ok(Some(children))
	}

	async fn try_get_build_log(
		&self,
		id: &build::Id,
	) -> Result<Option<BoxStream<'static, Result<Bytes>>>> {
		let request = self
			.request(http::Method::GET, &format!("/v1/builds/{id}/log"))
			.body(empty())
			.wrap_err("Failed to create the request.")?;
		let response = self
			.send(request)
			.await
			.wrap_err("Failed to send the request.")?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		let log = BodyStream::new(response.into_body())
			.filter_map(|frame| async {
				match frame.map(http_body::Frame::into_data) {
					Ok(Ok(bytes)) => Some(Ok(bytes)),
					Err(e) => Some(Err(e)),
					Ok(Err(_frame)) => None,
				}
			})
			.map_err(|error| error.wrap("Failed to read from the body."))
			.boxed();
		Ok(Some(log))
	}

	async fn try_get_build_result(&self, id: &build::Id) -> Result<Option<Result<Value>>> {
		let request = self
			.request(http::Method::GET, &format!("/v1/builds/{id}/result"))
			.body(empty())
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		let bytes = response
			.collect()
			.await
			.wrap_err("Failed to collect the response body.")?
			.to_bytes();
		let result =
			serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the response body.")?;
		Ok(Some(result))
	}

	async fn clean(&self) -> Result<()> {
		unimplemented!()
	}

	async fn create_login(&self) -> Result<Login> {
		let request = self
			.request(reqwest::Method::POST, "/v1/logins")
			.body(empty())
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		let bytes = response
			.collect()
			.await
			.wrap_err("Failed to collect the response body.")?
			.to_bytes();
		let response =
			serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the response body.")?;
		Ok(response)
	}

	async fn get_login(&self, id: &Id) -> Result<Option<Login>> {
		let request = self
			.request(reqwest::Method::GET, &format!("/v1/logins/{id}"))
			.body(empty())
			.wrap_err("Failed to create the request.")?;
		let response = self
			.send(request)
			.await
			.wrap_err("Failed to send the request.")?;
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		let bytes = response
			.collect()
			.await
			.wrap_err("Failed to collect the response body.")?
			.to_bytes();
		let response =
			serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the response body.")?;
		Ok(response)
	}

	async fn publish_package(&self, id: &package::Id) -> Result<()> {
		let request = self
			.request(reqwest::Method::POST, &format!("/v1/packages/{id}"))
			.body(empty())
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		Ok(())
	}

	async fn search_packages(&self, query: &str) -> Result<Vec<package::SearchResult>> {
		let path = &format!("/v1/packages/search?query={query}");
		let request = self
			.request(reqwest::Method::GET, path)
			.body(empty())
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		let bytes = response
			.collect()
			.await
			.wrap_err("Failed to collect the response body.")?
			.to_bytes();
		let response =
			serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the response body.")?;
		Ok(response)
	}

	async fn get_current_user(&self) -> Result<user::User> {
		let request = self
			.request(reqwest::Method::GET, "/v1/user")
			.body(empty())
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		let bytes = response
			.collect()
			.await
			.wrap_err("Failed to collect the response body.")?
			.to_bytes();
		let response =
			serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the response body.")?;
		Ok(response)
	}
}
