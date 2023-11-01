use crate::{
	build, object, package, status, target, tracker::Tracker, user, user::Login, Client, Handle,
	Id, Result, Value, Wrap, WrapErr,
};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{stream::BoxStream, StreamExt, TryStreamExt};
use http_body_util::{BodyExt, BodyStream};
use std::{
	os::unix::prelude::OsStrExt,
	path::Path,
	sync::{Arc, Weak},
};
use tangram_error::return_error;
use tangram_util::{
	http::{empty, full, Incoming, Outgoing},
	net::{Addr, Inet},
};
use tokio::{
	io::AsyncBufReadExt,
	net::{TcpStream, UnixStream},
};
use tokio_stream::wrappers::LinesStream;
use tokio_util::io::StreamReader;

#[derive(Debug, Clone)]
pub struct Remote {
	inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
	addr: Addr,
	file_descriptor_semaphore: tokio::sync::Semaphore,
	sender: tokio::sync::RwLock<Option<hyper::client::conn::http2::SendRequest<Outgoing>>>,
	tls: bool,
	token: std::sync::RwLock<Option<String>>,
}

pub struct Builder {
	addr: Addr,
	tls: Option<bool>,
	token: Option<String>,
}

impl Handle for Weak<Inner> {
	fn upgrade(&self) -> Option<Box<dyn Client>> {
		self.upgrade()
			.map(|inner| Box::new(Remote { inner }) as Box<dyn Client>)
	}
}

impl Remote {
	fn new(addr: Addr, tls: bool, token: Option<String>) -> Self {
		let file_descriptor_semaphore = tokio::sync::Semaphore::new(16);
		let sender = tokio::sync::RwLock::new(None);
		let token = std::sync::RwLock::new(token);
		let inner = Arc::new(Inner {
			addr,
			file_descriptor_semaphore,
			sender,
			tls,
			token,
		});
		Self { inner }
	}

	pub async fn disconnect(&self) -> Result<()> {
		*self.inner.sender.write().await = None;
		Ok(())
	}

	pub async fn connect(&self) -> Result<()> {
		self.sender().await.map(|_| ())
	}

	async fn sender(&self) -> Result<hyper::client::conn::http2::SendRequest<Outgoing>> {
		if let Some(sender) = self.inner.sender.read().await.as_ref().cloned() {
			return Ok(sender);
		}
		match &self.inner.addr {
			Addr::Inet(inet) if self.inner.tls => {
				self.connect_tcp_tls(inet).await?;
			},
			Addr::Inet(inet) => {
				self.connect_tcp(inet).await?;
			},
			Addr::Unix(path) => {
				self.connect_unix(path).await?;
			},
		}
		Ok(self.inner.sender.read().await.as_ref().cloned().unwrap())
	}

	async fn connect_tcp_tls(&self, inet: &Inet) -> Result<()> {
		let mut sender_guard = self.inner.sender.write().await;
		let stream = TcpStream::connect(inet.to_string())
			.await
			.wrap_err("Failed to create the TCP connection.")?;
		let mut root_cert_store = rustls::RootCertStore::empty();
		root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
		let config = rustls::ClientConfig::builder()
			.with_safe_defaults()
			.with_root_certificates(root_cert_store)
			.with_no_client_auth();
		let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
		let server_name = rustls::ServerName::try_from(inet.host.to_string().as_str())
			.wrap_err("Failed to create the server name.")?;
		let stream = connector
			.connect(server_name, stream)
			.await
			.wrap_err("Failed to connect.")?;
		let executor = hyper_util::rt::TokioExecutor::new();
		let io = hyper_util::rt::TokioIo::new(stream);
		let (mut sender, connection) = hyper::client::conn::http2::handshake(executor, io)
			.await
			.wrap_err("Failed to perform the HTTP handshake..")?;
		tokio::spawn(connection);
		sender
			.ready()
			.await
			.wrap_err("Failed to ready the sender.")?;
		sender_guard.replace(sender);
		Ok(())
	}

	async fn connect_tcp(&self, inet: &Inet) -> Result<()> {
		let mut sender_guard = self.inner.sender.write().await;
		let stream = TcpStream::connect(inet.to_string())
			.await
			.wrap_err("Failed to create the TCP connection.")?;
		let executor = hyper_util::rt::TokioExecutor::new();
		let io = hyper_util::rt::TokioIo::new(stream);
		let (mut sender, connection) = hyper::client::conn::http2::handshake(executor, io)
			.await
			.wrap_err("Failed to perform the HTTP handshake.")?;
		tokio::spawn(connection);
		sender
			.ready()
			.await
			.wrap_err("Failed to ready the sender.")?;
		sender_guard.replace(sender);
		Ok(())
	}

	async fn connect_unix(&self, path: &Path) -> Result<()> {
		let mut sender_guard = self.inner.sender.write().await;
		let stream = UnixStream::connect(path)
			.await
			.wrap_err("Failed to connect to the socket.")?;
		let executor = hyper_util::rt::TokioExecutor::new();
		let io = hyper_util::rt::TokioIo::new(stream);
		let (mut sender, connection) = hyper::client::conn::http2::handshake(executor, io)
			.await
			.wrap_err("Failed to perform the HTTP handshake.")?;
		tokio::spawn(connection);
		sender
			.ready()
			.await
			.wrap_err("Failed to ready the sender.")?;
		sender_guard.replace(sender);
		Ok(())
	}

	fn request(&self, method: http::Method, path: &str) -> http::request::Builder {
		let uri = match &self.inner.addr {
			Addr::Inet(url) => format!("{}{}", url, path.strip_prefix('/').unwrap()),
			Addr::Unix(_) => path.into(),
		};
		http::request::Builder::default().uri(uri).method(method)
	}

	async fn send(
		&self,
		request: http::request::Request<Outgoing>,
	) -> Result<http::Response<Incoming>> {
		self.sender()
			.await?
			.send_request(request)
			.await
			.wrap_err("Failed to send the request.")
	}
}

#[async_trait]
impl Client for Remote {
	fn clone_box(&self) -> Box<dyn Client> {
		Box::new(self.clone())
	}

	fn downgrade_box(&self) -> Box<dyn Handle> {
		Box::new(Arc::downgrade(&self.inner))
	}

	fn is_local(&self) -> bool {
		self.inner.addr.is_local()
	}

	fn path(&self) -> Option<&std::path::Path> {
		None
	}

	fn set_token(&self, token: Option<String>) {
		*self.inner.token.write().unwrap() = token;
	}

	fn file_descriptor_semaphore(&self) -> &tokio::sync::Semaphore {
		&self.inner.file_descriptor_semaphore
	}

	async fn status(&self) -> Result<status::Status> {
		let request = self
			.request(http::Method::GET, "/v1/status")
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
		let status = serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the body.")?;
		Ok(status)
	}

	async fn stop(&self) -> Result<()> {
		let request = self
			.request(http::Method::POST, "/v1/stop")
			.body(empty())
			.wrap_err("Failed to create the request.")?;
		self.send(request).await.ok();
		Ok(())
	}

	async fn clean(&self) -> Result<()> {
		let request = self
			.request(http::Method::POST, "/v1/clean")
			.body(empty())
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		Ok(())
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

	async fn try_get_tracker(&self, path: &Path) -> Result<Option<Tracker>> {
		let path = urlencoding::encode_binary(path.as_os_str().as_bytes());
		let request = self
			.request(http::Method::GET, &format!("/v1/trackers/{path}"))
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
		let tracker = serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the body.")?;
		Ok(Some(tracker))
	}

	async fn set_tracker(&self, path: &Path, tracker: &Tracker) -> Result<()> {
		let path = urlencoding::encode_binary(path.as_os_str().as_bytes());
		let body = serde_json::to_vec(&tracker).wrap_err("Failed to serialize the body.")?;
		let request = self
			.request(reqwest::Method::PATCH, &format!("/v1/trackers/{path}"))
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

	async fn try_get_build_target(&self, id: &build::Id) -> Result<Option<target::Id>> {
		let request = self
			.request(http::Method::POST, &format!("/v1/builds/{id}/target"))
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

	async fn try_get_build_queue_item(&self) -> Result<Option<build::Id>> {
		let request = self
			.request(http::Method::GET, "/v1/builds/queue")
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
		let build_id =
			serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the response body.")?;
		Ok(Some(build_id))
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

	async fn add_build_child(&self, build_id: &build::Id, child_id: &build::Id) -> Result<()> {
		let body = serde_json::to_vec(&child_id).wrap_err("Failed to serialize the body.")?;
		let request = self
			.request(
				http::Method::POST,
				&format!("/v1/builds/${build_id}/children"),
			)
			.body(full(body))
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(());
		}
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		Ok(())
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

	async fn add_build_log(&self, build_id: &build::Id, bytes: Bytes) -> Result<()> {
		let body = bytes;
		let request = self
			.request(http::Method::POST, &format!("/v1/builds/${build_id}/log"))
			.body(full(body))
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(());
		}
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		Ok(())
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

	async fn set_build_result(&self, build_id: &build::Id, result: Result<Value>) -> Result<()> {
		let result = match result {
			Ok(value) => Ok(value.data(self).await?),
			Err(error) => Err(error),
		};
		let body = serde_json::to_vec(&result).wrap_err("Failed to serialize the body.")?;
		let request = self
			.request(http::Method::POST, &format!("/v1/builds/${build_id}/log"))
			.body(full(body))
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(());
		}
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		Ok(())
	}

	async fn finish_build(&self, id: &build::Id) -> Result<()> {
		let request = self
			.request(http::Method::POST, &format!("/v1/builds/${id}/finish"))
			.body(empty())
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(());
		}
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}

		Ok(())
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
			.request(
				reqwest::Method::POST,
				&format!("/v1/registry/packages/{id}"),
			)
			.body(empty())
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		Ok(())
	}

	async fn search_packages(&self, query: &str) -> Result<Vec<package::SearchResult>> {
		let path = &format!("/v1/registry/packages/search?query={query}");
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

impl Builder {
	#[must_use]
	pub fn new(addr: Addr) -> Self {
		Self {
			addr,
			tls: None,
			token: None,
		}
	}

	#[must_use]
	pub fn tls(mut self, tls: bool) -> Self {
		self.tls = Some(tls);
		self
	}

	#[must_use]
	pub fn token(mut self, token: Option<String>) -> Self {
		self.token = token;
		self
	}

	#[must_use]
	pub fn build(self) -> Remote {
		let tls = self.tls.unwrap_or(false);
		Remote::new(self.addr, tls, self.token)
	}
}
