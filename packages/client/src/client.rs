use crate::{
	build, directory, lock, object, package, target, user, Dependency, Handle, Id, Status, System,
	User,
};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{stream::BoxStream, StreamExt, TryStreamExt};
use http_body_util::{BodyExt, BodyStream};
use std::{path::PathBuf, sync::Arc};
use tangram_error::{return_error, Error, Result, Wrap, WrapErr};
use tokio::{
	io::AsyncBufReadExt,
	net::{TcpStream, UnixStream},
};
use tokio_stream::wrappers::LinesStream;
use tokio_util::io::StreamReader;
use url::Url;

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
	user: Option<User>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Addr {
	Inet(Inet),
	Unix(PathBuf),
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Inet {
	pub host: Host,
	pub port: u16,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum Host {
	Ip(std::net::IpAddr),
	Domain(String),
}

pub struct Builder {
	addr: Addr,
	tls: Option<bool>,
	user: Option<User>,
}

type Incoming = hyper::body::Incoming;

type Outgoing = http_body_util::combinators::UnsyncBoxBody<
	::bytes::Bytes,
	Box<dyn std::error::Error + Send + Sync + 'static>,
>;

impl Client {
	fn new(addr: Addr, tls: bool, user: Option<User>) -> Self {
		let file_descriptor_semaphore = tokio::sync::Semaphore::new(16);
		let sender = tokio::sync::RwLock::new(None);
		let inner = Arc::new(Inner {
			addr,
			file_descriptor_semaphore,
			sender,
			tls,
			user,
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
			if sender.is_ready() {
				return Ok(sender);
			}
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

	async fn connect_tcp(&self, inet: &Inet) -> Result<()> {
		let mut sender_guard = self.inner.sender.write().await;

		// Connect via TCP.
		let stream = TcpStream::connect(inet.to_string())
			.await
			.wrap_err("Failed to create the TCP connection.")?;

		// Perform the HTTP handshake.
		let executor = hyper_util::rt::TokioExecutor::new();
		let io = hyper_util::rt::TokioIo::new(stream);
		let (mut sender, connection) = hyper::client::conn::http2::handshake(executor, io)
			.await
			.wrap_err("Failed to perform the HTTP handshake.")?;

		// Spawn the connection.
		tokio::spawn(async move {
			if let Err(error) = connection.await {
				tracing::error!(error = ?error, "The connection failed.");
			}
		});

		// Wait for the sender to be ready.
		sender
			.ready()
			.await
			.wrap_err("Failed to ready the sender.")?;

		// Replace the sender.
		sender_guard.replace(sender);

		Ok(())
	}

	async fn connect_tcp_tls(&self, inet: &Inet) -> Result<()> {
		let mut sender_guard = self.inner.sender.write().await;

		// Connect via TCP.
		let stream = TcpStream::connect(inet.to_string())
			.await
			.wrap_err("Failed to create the TCP connection.")?;

		// Create the connector.
		let mut root_store = rustls::RootCertStore::empty();
		root_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
			rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
				ta.subject,
				ta.spki,
				ta.name_constraints,
			)
		}));
		let mut config = rustls::ClientConfig::builder()
			.with_safe_defaults()
			.with_root_certificates(root_store)
			.with_no_client_auth();
		config.alpn_protocols = vec!["h2".into()];
		let connector = tokio_rustls::TlsConnector::from(Arc::new(config));

		// Create the server name.
		let server_name = rustls::ServerName::try_from(inet.host.to_string().as_str())
			.wrap_err("Failed to create the server name.")?;

		// Connect via TLS.
		let stream = connector
			.connect(server_name, stream)
			.await
			.wrap_err("Failed to connect.")?;

		// Verify the negotiated protocol.
		if !stream
			.get_ref()
			.1
			.alpn_protocol()
			.map(|protocol| protocol == b"h2")
			.unwrap_or_default()
		{
			return_error!("Failed to negotiate the HTTP/2 protocol.");
		}

		// Perform the HTTP handshake.
		let executor = hyper_util::rt::TokioExecutor::new();
		let io = hyper_util::rt::TokioIo::new(stream);
		let (mut sender, connection) = hyper::client::conn::http2::handshake(executor, io)
			.await
			.wrap_err("Failed to perform the HTTP handshake..")?;

		// Spawn the connection.
		tokio::spawn(async move {
			if let Err(error) = connection.await {
				tracing::error!(error = ?error, "The connection failed.");
			}
		});

		// Wait for the sender to be ready.
		sender
			.ready()
			.await
			.wrap_err("Failed to ready the sender.")?;

		// Replace the sender.
		sender_guard.replace(sender);

		Ok(())
	}

	async fn connect_unix(&self, path: &std::path::Path) -> Result<()> {
		let mut sender_guard = self.inner.sender.write().await;

		// Connect via UNIX.
		let stream = UnixStream::connect(path)
			.await
			.wrap_err("Failed to connect to the socket.")?;

		// Perform the HTTP handshake.
		let executor = hyper_util::rt::TokioExecutor::new();
		let io = hyper_util::rt::TokioIo::new(stream);
		let (mut sender, connection) = hyper::client::conn::http2::handshake(executor, io)
			.await
			.wrap_err("Failed to perform the HTTP handshake.")?;

		// Spawn the connection.
		tokio::spawn(async move {
			if let Err(error) = connection.await {
				tracing::error!(error = ?error, "The connection failed.");
			}
		});

		// Wait for the sender to be ready.
		sender
			.ready()
			.await
			.wrap_err("Failed to ready the sender.")?;

		// Replace the sender.
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
impl Handle for Client {
	fn clone_box(&self) -> Box<dyn Handle> {
		Box::new(self.clone())
	}

	fn file_descriptor_semaphore(&self) -> &tokio::sync::Semaphore {
		&self.inner.file_descriptor_semaphore
	}

	async fn status(&self) -> Result<Status> {
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

	async fn get_object_exists(&self, id: &object::Id) -> Result<bool> {
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

	async fn try_get_object(&self, id: &object::Id) -> Result<Option<Bytes>> {
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
		id: &object::Id,
		bytes: &Bytes,
	) -> Result<Result<(), Vec<object::Id>>> {
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

	async fn try_get_build_for_target(&self, id: &target::Id) -> Result<Option<build::Id>> {
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

	async fn get_or_create_build_for_target(
		&self,
		user: Option<&User>,
		id: &target::Id,
		depth: u64,
		retry: build::Retry,
	) -> Result<build::Id> {
		#[derive(serde::Serialize)]
		struct SearchParams {
			#[serde(default)]
			depth: u64,
			#[serde(default)]
			retry: build::Retry,
		}
		let search_params = SearchParams { depth, retry };
		let search_params = serde_urlencoded::to_string(search_params)
			.wrap_err("Failed to serialize the search params.")?;
		let uri = format!("/v1/targets/{id}/build?{search_params}");
		let mut request = http::request::Builder::default()
			.method(http::Method::POST)
			.uri(uri);
		let user = user.or(self.inner.user.as_ref());
		if let Some(token) = user.and_then(|user| user.token.as_ref()) {
			request = request.header(http::header::AUTHORIZATION, format!("Bearer {token}"));
		}
		let request = request
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

	async fn get_build_from_queue(
		&self,
		user: Option<&User>,
		systems: Option<Vec<System>>,
	) -> Result<build::queue::Item> {
		let uri = if let Some(systems) = systems {
			let systems = systems
				.iter()
				.map(ToString::to_string)
				.collect::<Vec<_>>()
				.join(",");
			format!("/v1/builds/queue?systems={systems}")
		} else {
			"/v1/builds/queue".to_owned()
		};
		let mut request = http::request::Builder::default()
			.method(http::Method::GET)
			.uri(uri);
		let user = user.or(self.inner.user.as_ref());
		if let Some(token) = user.and_then(|user| user.token.as_ref()) {
			request = request.header(http::header::AUTHORIZATION, format!("Bearer {token}"));
		}
		let request = request
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
		let item =
			serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the response body.")?;
		Ok(item)
	}

	async fn try_get_build_target(&self, id: &build::Id) -> Result<Option<target::Id>> {
		let request = http::request::Builder::default()
			.method(http::Method::GET)
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

	async fn try_get_build_children(
		&self,
		id: &build::Id,
	) -> Result<Option<BoxStream<'static, Result<build::Id>>>> {
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
		user: Option<&User>,
		build_id: &build::Id,
		child_id: &build::Id,
	) -> Result<()> {
		let mut request = http::request::Builder::default()
			.method(http::Method::POST)
			.uri(format!("/v1/builds/{build_id}/children"));
		let user = user.or(self.inner.user.as_ref());
		if let Some(token) = user.and_then(|user| user.token.as_ref()) {
			request = request.header(http::header::AUTHORIZATION, format!("Bearer {token}"));
		}
		let body = serde_json::to_vec(&child_id).wrap_err("Failed to serialize the body.")?;
		let request = request
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

	async fn add_build_log(&self, user: Option<&User>, id: &build::Id, bytes: Bytes) -> Result<()> {
		let mut request = http::request::Builder::default()
			.method(http::Method::POST)
			.uri(format!("/v1/builds/{id}/log"));
		let user = user.or(self.inner.user.as_ref());
		if let Some(token) = user.and_then(|user| user.token.as_ref()) {
			request = request.header(http::header::AUTHORIZATION, format!("Bearer {token}"));
		}
		let body = bytes;
		let request = request
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

	async fn try_get_build_outcome(&self, id: &build::Id) -> Result<Option<build::Outcome>> {
		let request = http::request::Builder::default()
			.method(http::Method::GET)
			.uri(format!("/v1/builds/{id}/outcome"))
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
		let outcome =
			serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the response body.")?;
		Ok(Some(outcome))
	}

	async fn cancel_build(&self, user: Option<&User>, id: &build::Id) -> Result<()> {
		let mut request = http::request::Builder::default()
			.method(http::Method::POST)
			.uri(format!("/v1/builds/{id}/cancel"));
		let user = user.or(self.inner.user.as_ref());
		if let Some(token) = user.and_then(|user| user.token.as_ref()) {
			request = request.header(http::header::AUTHORIZATION, format!("Bearer {token}"));
		}
		let request = request
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

	async fn finish_build(
		&self,
		user: Option<&User>,
		id: &build::Id,
		outcome: build::Outcome,
	) -> Result<()> {
		let mut request = http::request::Builder::default()
			.method(http::Method::POST)
			.uri(format!("/v1/builds/{id}/finish"));
		let user = user.or(self.inner.user.as_ref());
		if let Some(token) = user.and_then(|user| user.token.as_ref()) {
			request = request.header(http::header::AUTHORIZATION, format!("Bearer {token}"));
		}
		let outcome = outcome.data(self).await?;
		let body = serde_json::to_vec(&outcome).wrap_err("Failed to serialize the body.")?;
		let request = request
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

	async fn create_package_and_lock(
		&self,
		_dependency: &Dependency,
	) -> Result<(directory::Id, lock::Id)> {
		return_error!("Unsupported.");
	}

	async fn search_packages(&self, query: &str) -> Result<Vec<String>> {
		let request = http::request::Builder::default()
			.method(http::Method::GET)
			.uri(format!("/v1/packages/search?query={query}"))
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

	async fn try_get_package(&self, dependency: &Dependency) -> Result<Option<directory::Id>> {
		let request = http::request::Builder::default()
			.method(http::Method::GET)
			.uri(format!("/v1/packages/{dependency}"))
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

	async fn try_get_package_versions(
		&self,
		dependency: &Dependency,
	) -> Result<Option<Vec<String>>> {
		let request = http::request::Builder::default()
			.method(http::Method::GET)
			.uri(format!("/v1/packages/{dependency}/versions"))
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
		let id =
			serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the response body.")?;
		Ok(Some(id))
	}

	async fn try_get_package_metadata(
		&self,
		dependency: &Dependency,
	) -> Result<Option<package::Metadata>> {
		let request = http::request::Builder::default()
			.method(http::Method::GET)
			.uri(format!("/v1/packages/{dependency}/metadata"))
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

	async fn try_get_package_dependencies(
		&self,
		dependency: &Dependency,
	) -> Result<Option<Vec<Dependency>>> {
		let request = http::request::Builder::default()
			.method(http::Method::GET)
			.uri(format!("/v1/packages/{dependency}/dependencies"))
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

	async fn publish_package(&self, user: Option<&User>, id: &directory::Id) -> Result<()> {
		let mut request = http::request::Builder::default()
			.method(http::Method::POST)
			.uri("/v1/packages");
		let user = user.or(self.inner.user.as_ref());
		if let Some(token) = user.and_then(|user| user.token.as_ref()) {
			request = request.header(http::header::AUTHORIZATION, format!("Bearer {token}"));
		}
		let body = serde_json::to_vec(&id).wrap_err("Failed to serialize the body.")?;
		let request = request
			.body(full(body))
			.wrap_err("Failed to create the request.")?;
		let response = self.send(request).await?;
		if !response.status().is_success() {
			return_error!("Expected the response's status to be success.");
		}
		Ok(())
	}

	async fn create_login(&self) -> Result<user::Login> {
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

	async fn get_login(&self, id: &Id) -> Result<Option<user::Login>> {
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

	async fn get_user_for_token(&self, token: &str) -> Result<Option<user::User>> {
		let request = http::request::Builder::default()
			.method(http::Method::GET)
			.uri("/v1/user")
			.header(http::header::AUTHORIZATION, format!("Bearer {token}"))
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

impl Addr {
	#[must_use]
	pub fn is_local(&self) -> bool {
		match &self {
			Addr::Inet(inet) => match &inet.host {
				Host::Domain(domain) => domain == "localhost",
				Host::Ip(ip) => ip.is_loopback(),
			},
			Addr::Unix(_) => true,
		}
	}
}

impl std::fmt::Display for Addr {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Addr::Inet(inet) => write!(f, "{inet}"),
			Addr::Unix(path) => write!(f, "unix:{}", path.display()),
		}
	}
}

impl std::fmt::Display for Inet {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}", self.host, self.port)
	}
}

impl std::fmt::Display for Host {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Host::Ip(ip) => write!(f, "{ip}"),
			Host::Domain(domain) => write!(f, "{domain}"),
		}
	}
}

impl std::str::FromStr for Addr {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let mut parts = s.splitn(2, ':');
		let host = parts
			.next()
			.wrap_err("Expected a host.")?
			.parse()
			.wrap_err("Failed to parse the host.")?;
		if matches!(&host, Host::Domain(hostname) if hostname == "unix") {
			let path = parts.next().wrap_err("Expected a path.")?;
			Ok(Addr::Unix(path.into()))
		} else {
			let port = parts
				.next()
				.wrap_err("Expected a port.")?
				.parse()
				.wrap_err("Failed to parse the port.")?;
			Ok(Addr::Inet(Inet { host, port }))
		}
	}
}

impl std::str::FromStr for Host {
	type Err = std::net::AddrParseError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		if let Ok(ip) = s.parse() {
			Ok(Host::Ip(ip))
		} else {
			Ok(Host::Domain(s.to_string()))
		}
	}
}

impl TryFrom<Url> for Addr {
	type Error = Error;

	fn try_from(value: Url) -> Result<Self, Self::Error> {
		let host = value
			.host_str()
			.wrap_err("Invalid URL.")?
			.parse()
			.wrap_err("Invalid URL.")?;
		let port = value.port_or_known_default().wrap_err("Invalid URL.")?;
		Ok(Addr::Inet(Inet { host, port }))
	}
}
impl Builder {
	#[must_use]
	pub fn new(addr: Addr) -> Self {
		Self {
			addr,
			tls: None,
			user: None,
		}
	}

	#[must_use]
	pub fn tls(mut self, tls: bool) -> Self {
		self.tls = Some(tls);
		self
	}

	#[must_use]
	pub fn user(mut self, user: Option<User>) -> Self {
		self.user = user;
		self
	}

	#[must_use]
	pub fn build(self) -> Client {
		let tls = self.tls.unwrap_or(false);
		Client::new(self.addr, tls, self.user)
	}
}

#[must_use]
pub fn empty() -> Outgoing {
	http_body_util::Empty::new()
		.map_err(Into::into)
		.boxed_unsync()
}

#[must_use]
pub fn full(chunk: impl Into<::bytes::Bytes>) -> Outgoing {
	http_body_util::Full::new(chunk.into())
		.map_err(Into::into)
		.boxed_unsync()
}
