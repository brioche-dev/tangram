use crate::{
	net::{Addr, Inet},
	util::{empty, full, Incoming, Outgoing},
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
use tangram_client as tg;
use tangram_error::return_error;
use tg::{Result, Wrap, WrapErr};
use tokio::{
	io::AsyncBufReadExt,
	net::{TcpStream, UnixStream},
};
use tokio_stream::wrappers::LinesStream;
use tokio_util::io::StreamReader;

#[derive(Debug, Clone)]
pub struct Client {
	inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
	addr: Addr,
	file_descriptor_semaphore: tokio::sync::Semaphore,
	sender: tokio::sync::RwLock<Option<hyper::client::conn::http2::SendRequest<Outgoing>>>,
	tls: bool,
}

#[derive(Debug)]
pub struct Handle(Weak<Inner>);

pub struct Builder {
	addr: Addr,
	tls: Option<bool>,
}

impl tg::Handle for Handle {
	fn upgrade(&self) -> Option<Box<dyn tg::Client>> {
		self.0
			.upgrade()
			.map(|inner| Box::new(Client { inner }) as Box<dyn tg::Client>)
	}
}

impl Client {
	fn new(addr: Addr, tls: bool) -> Self {
		let file_descriptor_semaphore = tokio::sync::Semaphore::new(16);
		let sender = tokio::sync::RwLock::new(None);
		let inner = Arc::new(Inner {
			addr,
			file_descriptor_semaphore,
			sender,
			tls,
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
impl tg::Client for Client {
	fn clone_box(&self) -> Box<dyn tg::Client> {
		Box::new(self.clone())
	}

	fn downgrade_box(&self) -> Box<dyn tg::Handle> {
		Box::new(Handle(Arc::downgrade(&self.inner)))
	}

	fn is_local(&self) -> bool {
		self.inner.addr.is_local()
	}

	fn path(&self) -> Option<&std::path::Path> {
		None
	}

	fn file_descriptor_semaphore(&self) -> &tokio::sync::Semaphore {
		&self.inner.file_descriptor_semaphore
	}

	async fn status(&self) -> Result<tg::status::Status> {
		let request = http::request::Builder::default()
			.method(http::Method::GET)
			.uri("/v1/status")
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
		let request = http::request::Builder::default()
			.method(http::Method::POST)
			.uri("/v1/stop")
			.body(empty())
			.wrap_err("Failed to create the request.")?;
		self.send(request).await.ok();
		Ok(())
	}

	async fn clean(&self) -> Result<()> {
		let request = http::request::Builder::default()
			.method(http::Method::POST)
			.uri("/v1/clean")
			.body(empty())
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		Ok(())
	}

	async fn get_object_exists(&self, id: &tg::object::Id) -> Result<bool> {
		let request = http::request::Builder::default()
			.method(http::Method::HEAD)
			.uri(format!("/v1/objects/{id}"))
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

	async fn try_get_object(&self, id: &tg::object::Id) -> Result<Option<Bytes>> {
		let request = http::request::Builder::default()
			.method(http::Method::GET)
			.uri(format!("/v1/objects/{id}"))
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

	async fn try_put_object(
		&self,
		id: &tg::object::Id,
		bytes: &Bytes,
	) -> Result<Result<(), Vec<tg::object::Id>>> {
		let body = full(bytes.clone());
		let request = http::request::Builder::default()
			.method(http::Method::PUT)
			.uri(format!("/v1/objects/{id}"))
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

	async fn try_get_tracker(&self, path: &Path) -> Result<Option<tg::Tracker>> {
		let path = urlencoding::encode_binary(path.as_os_str().as_bytes());
		let request = http::request::Builder::default()
			.method(http::Method::GET)
			.uri(format!("/v1/trackers/{path}"))
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

	async fn set_tracker(&self, path: &Path, tracker: &tg::Tracker) -> Result<()> {
		let path = urlencoding::encode_binary(path.as_os_str().as_bytes());
		let body = serde_json::to_vec(&tracker).wrap_err("Failed to serialize the body.")?;
		let request = http::request::Builder::default()
			.method(http::Method::PATCH)
			.uri(format!("/v1/trackers/{path}"))
			.body(full(body))
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		Ok(())
	}

	async fn try_get_build_for_target(&self, id: &tg::target::Id) -> Result<Option<tg::build::Id>> {
		let request = http::request::Builder::default()
			.method(http::Method::GET)
			.uri(format!("/v1/targets/{id}/build"))
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

	async fn get_or_create_build_for_target(&self, id: &tg::target::Id) -> Result<tg::build::Id> {
		let request = http::request::Builder::default()
			.method(http::Method::POST)
			.uri(format!("/v1/targets/{id}/build"))
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

	async fn try_get_build_target(&self, id: &tg::build::Id) -> Result<Option<tg::target::Id>> {
		let request = http::request::Builder::default()
			.method(http::Method::POST)
			.uri(format!("/v1/builds/{id}/target"))
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

	async fn try_get_build_queue_item(&self) -> Result<Option<tg::build::Id>> {
		let request = http::request::Builder::default()
			.method(http::Method::GET)
			.uri("/v1/builds/queue")
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
		id: &tg::build::Id,
	) -> Result<Option<BoxStream<'static, Result<tg::build::Id>>>> {
		let request = http::request::Builder::default()
			.method(http::Method::GET)
			.uri(format!("/v1/builds/{id}/children"))
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

	async fn add_build_child(
		&self,
		build_id: &tg::build::Id,
		child_id: &tg::build::Id,
	) -> Result<()> {
		let body = serde_json::to_vec(&child_id).wrap_err("Failed to serialize the body.")?;
		let request = http::request::Builder::default()
			.method(http::Method::POST)
			.uri(format!("/v1/builds/${build_id}/children"))
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
		id: &tg::build::Id,
	) -> Result<Option<BoxStream<'static, Result<Bytes>>>> {
		let request = http::request::Builder::default()
			.method(http::Method::GET)
			.uri(format!("/v1/builds/{id}/log"))
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

	async fn add_build_log(&self, build_id: &tg::build::Id, bytes: Bytes) -> Result<()> {
		let body = bytes;
		let request = http::request::Builder::default()
			.method(http::Method::POST)
			.uri(format!("/v1/builds/${build_id}/log"))
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

	async fn try_get_build_result(&self, id: &tg::build::Id) -> Result<Option<Result<tg::Value>>> {
		let request = http::request::Builder::default()
			.method(http::Method::GET)
			.uri(format!("/v1/builds/{id}/result"))
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

	async fn set_build_result(
		&self,
		build_id: &tg::build::Id,
		result: Result<tg::Value>,
	) -> Result<()> {
		let result = match result {
			Ok(value) => Ok(value.data(self).await?),
			Err(error) => Err(error),
		};
		let body = serde_json::to_vec(&result).wrap_err("Failed to serialize the body.")?;
		let request = http::request::Builder::default()
			.method(http::Method::POST)
			.uri(format!("/v1/builds/${build_id}/log"))
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

	async fn finish_build(&self, id: &tg::build::Id) -> Result<()> {
		let request = http::request::Builder::default()
			.method(http::Method::POST)
			.uri(format!("/v1/builds/${id}/finish"))
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

	async fn search_packages(&self, query: &str) -> Result<Vec<tg::Package>> {
		let request = http::request::Builder::default()
			.method(http::Method::GET)
			.uri(format!("/v1/registry/packages/search?query={query}"))
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

	async fn get_package(&self, name: &str) -> Result<Option<tg::Package>> {
		let request = http::request::Builder::default()
			.method(http::Method::GET)
			.uri(format!("/v1/registry/packages/{name}"))
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

	async fn get_package_version(
		&self,
		name: &str,
		version: &str,
	) -> Result<Option<tg::artifact::Id>> {
		let request = http::request::Builder::default()
			.method(http::Method::GET)
			.uri(format!("/v1/registry/packages/{name}/version/{version}"))
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

	async fn publish_package(&self, token: &str, id: &tg::artifact::Id) -> Result<()> {
		let body = serde_json::to_vec(&id).wrap_err("Failed to serialize the body.")?;
		let request = http::request::Builder::default()
			.method(http::Method::POST)
			.uri("/v1/registry/packages")
			.header(http::header::AUTHORIZATION, format!("Bearer {}", token))
			.body(full(body))
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		Ok(())
	}

	async fn create_login(&self) -> Result<tg::user::Login> {
		let request = http::request::Builder::default()
			.method(http::Method::POST)
			.uri("/v1/logins")
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

	async fn get_login(&self, id: &tg::Id) -> Result<Option<tg::user::Login>> {
		let request = http::request::Builder::default()
			.method(http::Method::GET)
			.uri(format!("/v1/logins/{id}"))
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

	async fn get_current_user(&self, _token: &str) -> Result<Option<tg::user::User>> {
		let request = http::request::Builder::default()
			.method(http::Method::GET)
			.uri("/v1/user")
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
		Self { addr, tls: None }
	}

	#[must_use]
	pub fn tls(mut self, tls: bool) -> Self {
		self.tls = Some(tls);
		self
	}

	#[must_use]
	pub fn build(self) -> Client {
		let tls = self.tls.unwrap_or(false);
		Client::new(self.addr, tls)
	}
}
