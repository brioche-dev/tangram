use hyper_util::rt::TokioIo;
use std::{path::PathBuf, str::FromStr};
use tangram_error::{return_error, Error, WrapErr};
use tokio::{
	io::{AsyncRead, AsyncWrite},
	net::{TcpListener, TcpStream, UnixListener, UnixStream},
};
use url::Url;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Addr {
	Inet(Url),
	Unix(PathBuf),
}

impl Addr {
	pub fn is_local(&self) -> bool {
		match &self {
			Addr::Inet(url) => match url.host().unwrap() {
				url::Host::Domain(domain) => domain == "localhost",
				url::Host::Ipv4(ip) => ip.is_loopback(),
				url::Host::Ipv6(ip) => ip.is_loopback(),
			},
			Addr::Unix(_) => true,
		}
	}
}

impl FromStr for Addr {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let url: Url = s.parse().wrap_err("Failed to parse the string as a URL.")?;
		url.try_into()
	}
}

impl TryFrom<Url> for Addr {
	type Error = Error;

	fn try_from(value: Url) -> Result<Self, Self::Error> {
		let scheme = value.scheme();
		let addr = match scheme {
			"http" | "https" => Addr::Inet(value),
			"unix" => Addr::Unix(value.path().into()),
			scheme => return_error!("Unknown URI scheme {scheme:#?}."),
		};
		Ok(addr)
	}
}

#[cfg(test)]
mod tests {
	use crate::addr::Addr;

	#[test]
	fn parse() {
		assert_eq!(
			"http://api.tangram.dev".parse::<Addr>().unwrap(),
			Addr::Inet("http://api.tangram.dev".parse().unwrap())
		);
		assert_eq!(
			"unix:///.tangram/socket".parse::<Addr>().unwrap(),
			Addr::Unix("/.tangram/socket".into())
		);
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
			Addr::Inet(url) => {
				let addr = format!("{}:{}", url.host().unwrap(), url.port().unwrap_or(8476));
				Ok(Self::Tcp(TcpListener::bind(addr).await?))
			},
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
