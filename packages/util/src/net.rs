use hyper_util::rt::TokioIo;
use std::{path::PathBuf, str::FromStr};
use tangram_error::{Error, WrapErr};
use tokio::{
	io::{AsyncRead, AsyncWrite},
	net::{TcpListener, TcpStream, UnixListener, UnixStream},
};

#[derive(Clone, Debug)]
pub enum Addr {
	Inet(Inet),
	Unix(PathBuf),
}

#[derive(Clone, Debug)]
pub struct Inet {
	pub host: Host,
	pub port: u16,
}

#[derive(Clone, Debug)]
pub enum Host {
	Ip(std::net::IpAddr),
	Domain(String),
}

impl Addr {
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

impl FromStr for Addr {
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

pub enum Listener {
	Tcp(TcpListener),
	Socket(UnixListener),
}

pub trait Stream: AsyncRead + AsyncWrite + Send + Unpin + 'static {}

impl Stream for TcpStream {}

impl Stream for UnixStream {}

impl Listener {
	pub async fn bind(addr: &Addr) -> std::io::Result<Self> {
		match addr {
			Addr::Inet(inet) => Ok(Self::Tcp(TcpListener::bind(inet.to_string()).await?)),
			Addr::Unix(path) => Ok(Self::Socket(UnixListener::bind(path)?)),
		}
	}

	pub async fn accept(&self) -> std::io::Result<TokioIo<Box<dyn Stream>>> {
		match self {
			Listener::Tcp(listener) => {
				let (stream, _) = listener.accept().await?;
				Ok(TokioIo::new(Box::new(stream)))
			},
			Listener::Socket(listener) => {
				let (stream, _) = listener.accept().await?;
				Ok(TokioIo::new(Box::new(stream)))
			},
		}
	}
}
