use std::io::BufReader;
use std::path::Path;
use std::sync::{RwLock, Weak};
use std::{path::PathBuf, sync::Arc};
use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::BoxStream;
use futures::{StreamExt, TryStreamExt};
use tangram_util::{bytes_stream, empty, full, BodyExt, Incoming, Outgoing, ResponseExt};
use tokio::io::AsyncBufReadExt;
use tokio::net::{TcpStream, UnixStream};
use tokio_rustls::{TlsConnector, TlsStream};
use tokio_stream::wrappers::LinesStream;
use tokio_util::io::StreamReader;
use url::Url;
use crate::{
	build, error, object, package, target, user, user::Login, value, Artifact, Client, Handle, Id,
	Package, Result, Value, Wrap, WrapErr,
};

#[derive(Debug, Clone)]
pub struct Hyper {
	state: Arc<State>,
}

#[derive(Debug)]
struct State {
	addr: Addr,
	sender: hyper::client::conn::http2::SendRequest<Outgoing>,
	file_descriptor_semaphore: tokio::sync::Semaphore,
	token: std::sync::RwLock<Option<String>>,
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

#[derive(Debug)]
pub enum Addr {
	Inet(Url),
	Socket(PathBuf),
}

impl Handle for Weak<State> {
	fn upgrade(&self) -> Option<Box<dyn Client>> {
		self.upgrade()
			.map(|state| Box::new(Hyper { state }) as Box<dyn Client>)
	}
}

impl Hyper {
	pub async fn new(addr: Addr, token: Option<String>) -> Result<Self> {
		let is_local = match &addr {
			Addr::Inet(url) => url.host().map_or(false, |host| match host {
				url::Host::Domain(domain) => domain == "localhost",
				url::Host::Ipv4(ip) => ip.is_loopback(),
				url::Host::Ipv6(ip) => ip.is_loopback(),
			}),
			Addr::Socket(_) => true,
		};

		let exec = hyper_util::rt::TokioExecutor::new();
		let sender = match &addr {
			Addr::Inet(url) if url.scheme() == "https" => {
				let stream = tcp_stream(url).await?;
				let stream = tls_stream(stream, url, None).await?;
				let io = hyper_util::rt::TokioIo::new(stream);
				let (sender, connection) = hyper::client::conn::http2::handshake(exec, io)
					.await
					.wrap_err("Failed to create hyper stream.")?;
				tokio::spawn(connection);
				sender
			},
			Addr::Inet(url) => {
				let stream = tcp_stream(url).await?;
				let io = hyper_util::rt::TokioIo::new(stream);
				let (sender, connection) = hyper::client::conn::http2::handshake(exec, io)
					.await
					.wrap_err("Failed to create hyper stream.")?;
				tokio::spawn(connection);
				sender
			},
			Addr::Socket(path) => {
				let stream = socket_stream(path).await?;
				let io = hyper_util::rt::TokioIo::new(stream);
				let (sender, connection) = hyper::client::conn::http2::handshake(exec, io)
					.await
					.wrap_err("Failed to create hyper stream.")?;
				tokio::spawn(connection);
				sender
			},
		};

		let file_descriptor_semaphore = tokio::sync::Semaphore::new(16);
		let token = RwLock::new(token);
		let state = Arc::new(State {
			addr,
			sender,
			file_descriptor_semaphore,
			token,
			is_local,
		});

		Ok(Self { state })
	}

	fn request(&self, method: http::Method, path: &str) -> http::request::Builder {
		let uri = match &self.state.addr {
			Addr::Inet(url) => format!("{}{}", url, path.strip_prefix('/').unwrap()),
			Addr::Socket(_) => path.into(),
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
			.wrap_err("Failed to send request.")
	}
}

async fn tcp_stream(url: &Url) -> Result<TcpStream> {
	let addr = format!("{}:{}", url.host_str().unwrap(), url.port().unwrap());
	let stream = TcpStream::connect(addr)
		.await
		.wrap_err("Failed to connect to url.")?;
	Ok(stream)
}

async fn socket_stream(path: &Path) -> Result<UnixStream> {
	let stream = UnixStream::connect(path)
		.await
		.wrap_err("Failed to connect to socket")?;
	Ok(stream)
}

async fn tls_stream(
	stream: TcpStream,
	url: &Url,
	cafile: Option<PathBuf>,
) -> Result<TlsStream<TcpStream>> {
	let mut root_cert_store = rustls::RootCertStore::empty();
	if let Some(cafile) = cafile {
		// Has to be io::BufRead.
		let mut pem =
			BufReader::new(std::fs::File::open(cafile).wrap_err("Failed to open cacert file")?);
		for cert in rustls_pemfile::certs(&mut pem) {
			let der = cert.wrap_err("Failed to parse cert.")?;
			root_cert_store.add(der).wrap_err("Failed to add der.")?;
		}
	} else {
		root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
	}

	let config = rustls::ClientConfig::builder()
		.with_safe_defaults()
		.with_root_certificates(root_cert_store)
		.with_no_client_auth();
	let connector = TlsConnector::from(Arc::new(config));

	let domain = url.domain().wrap_err("Missing domain for URL.")?;
	let domain = rustls::ServerName::try_from(domain).wrap_err("Failed to create server name.")?;
	let stream = connector
		.connect(domain, stream)
		.await
		.wrap_err("Failed to connect.")?;
	Ok(stream.into())
}

#[async_trait]
impl Client for Hyper {
	fn clone_box(&self) -> Box<dyn Client> {
		Box::new(self.clone())
	}

	fn downgrade_box(&self) -> Box<dyn Handle> {
		Box::new(Arc::downgrade(&self.state))
	}

	fn is_local(&self) -> bool {
		self.state.is_local
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
			.wrap_err("Failed to create request.")?;
		let response = self.send(request).await?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(false);
		}
		response
			.error_for_status()
			.map_err(|_| error!("Expected the status to be success."))?;
		Ok(true)
	}

	async fn try_get_object_bytes(&self, id: &object::Id) -> Result<Option<Vec<u8>>> {
		let request = self
			.request(http::Method::GET, &format!("/v1/objects/{id}"))
			.body(empty())
			.wrap_err("Failed to create request.")?;
		let response = self.send(request).await?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}
		let response = response
			.error_for_status()
			.map_err(|_| error!("Expected the status to be success."))?;
		let bytes = response
			.bytes()
			.await
			.wrap_err("Failed to get the response bytes.")?;
		Ok(Some(bytes.into()))
	}

	async fn try_put_object_bytes(
		&self,
		id: &object::Id,
		bytes: &[u8],
	) -> Result<Result<(), Vec<object::Id>>> {
		let body = full(bytes.to_owned());
		let request = self
			.request(http::Method::PUT, &format!("/v1/objects/{id}"))
			.body(body)
			.wrap_err("Failed to create request.")?;
		let response = self.send(request).await?;
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
			.map_err(|_| error!("Expected the status to be success."))?;
		Ok(Ok(()))
	}

	async fn try_get_artifact_for_path(&self, path: &Path) -> Result<Option<Artifact>> {
		let body = GetForPathBody { path: path.into() };
		let body = serde_json::to_vec(&body).wrap_err("Failed to serialize to json.")?;
		let request = self
			.request(http::Method::GET, "/v1/artifact/path")
			.body(full(body))
			.wrap_err("Failed to create request.")?;
		let response = self.send(request).await?;

		if response.status().is_success() {
			let id: Id = response
				.json()
				.await
				.wrap_err("Failed to deserialize the response as json.")?;
			let artifact = Artifact::with_id(id.try_into()?);
			Ok(Some(artifact))
		} else if response.status().as_u16() == 404 {
			Ok(None)
		} else {
			Err(response
				.error_for_status()
				.map_err(|_| error!("The response had a non-success status."))
				.unwrap_err())
		}
	}

	async fn set_artifact_for_path(&self, path: &Path, artifact: &Artifact) -> Result<()> {
		let path = path.into();
		let id = artifact.id(self).await?.into();
		let body = SetForPathBody { path, id };
		let body = serde_json::to_vec(&body).wrap_err("Failed to serialize to json.")?;
		let request = self
			.request(reqwest::Method::PUT, "/v1/artifact/path")
			.body(full(body))
			.wrap_err("Failed to create request.")?;
		self.send(request)
			.await?
			.error_for_status()
			.map_err(|_| error!("The response had a non-success status."))?;
		Ok(())
	}

	async fn try_get_package_for_path(&self, path: &Path) -> Result<Option<Package>> {
		let body = GetForPathBody { path: path.into() };
		let body = serde_json::to_vec(&body).wrap_err("Failed to serialize to json.")?;
		let request = self
			.request(reqwest::Method::GET, "/v1/package/path")
			.body(full(body))
			.wrap_err("Failed to create request.")?;
		let response = self.send(request).await?;
		if response.status().is_success() {
			let id: Id = response
				.json()
				.await
				.wrap_err("Failed to deserialize the reponse as json.")?;
			let package = Package::with_id(id.try_into()?);
			Ok(Some(package))
		} else if response.status().as_u16() == 404 {
			Ok(None)
		} else {
			Err(response
				.error_for_status()
				.map_err(|_| error!("The response had a non-success status."))
				.unwrap_err())
		}
	}

	async fn set_package_for_path(&self, path: &Path, package: &Package) -> Result<()> {
		let path = path.into();
		let id = package.id(self).await?.clone().into();
		let body = SetForPathBody { path, id };
		let body = serde_json::to_vec(&body).wrap_err("Failed to serialize to json.")?;
		let request = self
			.request(reqwest::Method::PUT, "/v1/package/path")
			.body(full(body))
			.wrap_err("Failed to create request.")?;
		self.send(request)
			.await?
			.error_for_status()
			.map_err(|_| error!("The response had a non-success status."))?;
		Ok(())
	}

	async fn try_get_build_for_target(&self, id: &target::Id) -> Result<Option<build::Id>> {
		let request = self
			.request(http::Method::GET, &format!("/v1/targets/{id}/build"))
			.body(empty())
			.wrap_err("Failed to create request.")?;
		let response = self.send(request).await?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}
		let response = response
			.error_for_status()
			.map_err(|_| error!("Expected the status to be success."))?;
		let bytes = response
			.bytes()
			.await
			.wrap_err("Failed to get the response body.")?;
		let id = serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the body.")?;
		Ok(Some(id))
	}

	async fn get_or_create_build_for_target(&self, id: &target::Id) -> Result<build::Id> {
		let request = self
			.request(http::Method::POST, &format!("/v1/targets/{id}/build"))
			.body(empty())
			.wrap_err("Failed to create request.")?;
		let response = self
			.send(request)
			.await?
			.error_for_status()
			.map_err(|_| error!("Expected the status to be success."))?;
		let bytes = response
			.bytes()
			.await
			.wrap_err("Failed to get the response body.")?;
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
			.wrap_err("Failed to create request.")?;
		let response = self.send(request).await?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}
		let response = response
			.error_for_status()
			.map_err(|_| error!("Expected the status to be success."))?;
		let stream = bytes_stream(response.into_body())
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
			.wrap_err("Failed to create request.")?;
		let response = self
			.send(request)
			.await
			.wrap_err("Failed to send the request.")?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}
		let response = response
			.error_for_status()
			.map_err(|_| error!("Expected the status to be success."))?;
		let log = bytes_stream(response.into_body())
			.map_err(|error| error.wrap("Failed to read from the response stream."))
			.boxed();
		Ok(Some(log))
	}

	async fn try_get_build_result(&self, id: &build::Id) -> Result<Option<Result<Value>>> {
		let request = self
			.request(http::Method::GET, &format!("/v1/builds/{id}/result"))
			.body(empty())
			.wrap_err("Failed to create request.")?;
		let response = self.send(request).await?;
		if response.status() == http::StatusCode::NOT_FOUND {
			return Ok(None);
		}
		let response = response
			.error_for_status()
			.map_err(|_| error!("Expected the status to be success."))?;
		let bytes = response
			.bytes()
			.await
			.wrap_err("Failed to read the response body.")?;
		let result = serde_json::from_slice::<Result<value::Data>>(&bytes)
			.wrap_err("Failed to deserialize the response body.")?
			.map(Into::into);
		Ok(Some(result))
	}

	async fn clean(&self) -> Result<()> {
		unimplemented!()
	}

	async fn create_login(&self) -> Result<Login> {
		let request = self
			.request(reqwest::Method::POST, "/v1/logins")
			.body(empty())
			.wrap_err("Failed to create request.")?;
		let response = self
			.send(request)
			.await?
			.error_for_status()
			.map_err(|_| error!("Expected the status to be success."))?;
		let response = response
			.json()
			.await
			.wrap_err("Failed to get the response JSON.")?;
		Ok(response)
	}

	async fn get_login(&self, id: &Id) -> Result<Option<Login>> {
		let request = self
			.request(reqwest::Method::GET, &format!("/v1/logins/{id}"))
			.body(empty())
			.wrap_err("Failed to create request.")?;
		let response = self
			.send(request)
			.await
			.wrap_err("Failed to send the request.")?
			.error_for_status()
			.map_err(|_| error!("Expected the status to be success."))?;
		let response = response
			.json()
			.await
			.wrap_err("Failed to get the response JSON.")?;
		Ok(response)
	}

	async fn publish_package(&self, id: &package::Id) -> Result<()> {
		let request = self
			.request(reqwest::Method::POST, &format!("/v1/packages/{id}"))
			.body(empty())
			.wrap_err("Failed to create request.")?;
		self.send(request)
			.await?
			.error_for_status()
			.map_err(|_| error!("Expected the status to be success."))?;
		Ok(())
	}

	async fn search_packages(&self, query: &str) -> Result<Vec<package::SearchResult>> {
		let path = &format!("/v1/packages/search?query={query}");
		let request = self
			.request(reqwest::Method::GET, path)
			.body(empty())
			.wrap_err("Failed to create request.")?;
		let response = self
			.send(request)
			.await?
			.error_for_status()
			.map_err(|_| error!("Expected the status to be success."))?;
		let response = response
			.json()
			.await
			.wrap_err("Failed to get the response JSON.")?;
		Ok(response)
	}

	async fn get_current_user(&self) -> Result<user::User> {
		let request = self
			.request(reqwest::Method::GET, "/v1/user")
			.body(empty())
			.wrap_err("Failed to create request.")?;
		let response = self
			.send(request)
			.await?
			.error_for_status()
			.map_err(|_| error!("Expected the status to be success."))?;
		let user = response
			.json()
			.await
			.wrap_err("Failed to get the response JSON.")?;
		Ok(user)
	}
}
