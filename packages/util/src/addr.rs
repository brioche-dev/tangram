use hyper_util::rt::TokioIo;
use std::{path::PathBuf, str::FromStr};
use tokio::{
	io::{AsyncRead, AsyncWrite},
	net::{TcpListener, TcpStream, UnixListener, UnixStream},
};
use url::Url;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Addr {
	Inet(Url),
	Socket(PathBuf),
}

impl FromStr for Addr {
	type Err = Box<dyn std::error::Error>; // TODO: real error.
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let url: Url = s.parse()?;
		url.try_into()
	}
}

impl TryFrom<Url> for Addr {
	type Error = Box<dyn std::error::Error>; // TODO: real error.
	fn try_from(value: Url) -> Result<Self, Self::Error> {
		let scheme = value.scheme();
		let addr = match scheme {
			"http" | "https" => Addr::Inet(value),
			"unix" => Addr::Socket(value.path().into()),
			scheme => Err(format!("Unknown URI scheme {scheme:#?}."))?,
		};
		Ok(addr)
	}
}

impl Addr {
	pub fn is_local(&self) -> bool {
		match &self {
			Addr::Inet(url) => match url.host().unwrap() {
				url::Host::Domain(domain) => domain == "localhost",
				url::Host::Ipv4(ip) => ip.is_loopback(),
				url::Host::Ipv6(ip) => ip.is_loopback(),
			},
			Addr::Socket(_) => true,
		}
	}
}

pub enum Listener {
	Tcp(TcpListener),
	Socket(UnixListener),
}

pub trait Stream: AsyncRead + AsyncWrite + Send + 'static + Unpin {}
impl Stream for TcpStream {}
impl Stream for UnixStream {}

impl Listener {
	pub async fn bind(addr: &Addr) -> std::io::Result<Self> {
		match addr {
			Addr::Inet(url) => {
				let addr = format!("{}:{}", url.host().unwrap(), url.port().unwrap_or(8476));
				Ok(Self::Tcp(TcpListener::bind(addr).await?))
			},
			Addr::Socket(path) => Ok(Self::Socket(UnixListener::bind(path)?)),
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

#[cfg(test)]
mod tests {
	use crate::addr::Addr;

	#[test]
	fn parse() {
		assert_eq!(
			"http://tangram.com/api".parse::<Addr>().unwrap(),
			Addr::Inet("http://tangram.com/api".parse().unwrap())
		);
		assert_eq!(
			"unix:///tmp/tangram.sock".parse::<Addr>().unwrap(),
			Addr::Socket("/tmp/tangram.sock".into())
		);
	}
}
