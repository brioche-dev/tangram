use std::io::BufReader;
use std::path::Path;
use std::sync::RwLock;
use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use derive_more::FromStr;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpStream, UnixStream};
use tokio_rustls::{TlsConnector, TlsStream};
use url::Url;

use crate::{Id, Result, WrapErr};

#[derive(Debug, Clone)]
pub struct Hyper {
	state: Arc<State>,
}

#[derive(Debug)]
struct State {
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

pub enum Addr {
	Url(Url),
	Socket(PathBuf),
}

trait ConnectionStream: AsyncRead + AsyncWrite + Send + Unpin + 'static {}
impl ConnectionStream for UnixStream {}
impl ConnectionStream for TcpStream {}
impl ConnectionStream for TlsStream<TcpStream> {}

pub type Outgoing = http_body_util::combinators::UnsyncBoxBody<
	::bytes::Bytes,
	Box<dyn std::error::Error + Send + Sync + 'static>,
>;

impl Hyper {
	pub async fn new(addr: Addr, token: Option<String>) -> Result<Self> {
		let is_local = match &addr {
			Addr::Url(url) => url.host().map_or(false, |host| match host {
				url::Host::Domain(domain) => domain == "localhost",
				url::Host::Ipv4(ip) => ip.is_loopback(),
				url::Host::Ipv6(ip) => ip.is_loopback(),
			}),
			Addr::Socket(_) => true,
		};

		let stream: Box<dyn ConnectionStream> = match &addr {
			Addr::Url(url) => {
				let stream = tcp_stream(&url).await?;
				if url.scheme() == "https" {
					// TODO: support cacertfile.
					let stream = tls_stream(stream, url, None).await?;
					Box::new(stream)
				} else {
					Box::new(stream)
				}
			},
			Addr::Socket(path) => {
				let stream = socket_stream(path).await?;
				Box::new(stream)
			},
		};

		let io = hyper_util::rt::TokioIo::new(stream);
		let exec = hyper_util::rt::TokioExecutor::new();
		let (sender, connection) = hyper::client::conn::http2::handshake(exec, io)
			.await
			.wrap_err("Failed to create hyper stream.")?;
		tokio::spawn(async move {
			let _ = connection.await;
		});

		let file_descriptor_semaphore = tokio::sync::Semaphore::new(16);
		let token = RwLock::new(token);
		let state = Arc::new(State {
			sender,
			token,
			is_local,
			file_descriptor_semaphore,
		});
		Ok(Self { state })
	}
}

async fn tcp_stream(url: &Url) -> Result<TcpStream> {
	let addr = SocketAddr::from_str(url.host_str().unwrap())
		.wrap_err("Failed to get socket addr from hostname.")?;
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
