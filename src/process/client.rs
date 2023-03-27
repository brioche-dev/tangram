use crate::{
	artifact,
	error::{Error, Result},
	util::{
		fs,
		http::{Request, Response},
	},
};

pub struct Client {
	path: fs::PathBuf,
}

impl Client {
	pub fn new() -> Result<Client> {
		let path = fs::PathBuf::from(std::env::var("TANGRAM_SOCKET").map_err(Error::other)?);
		Ok(Client { path })
	}

	async fn request(&self, request: Request) -> Result<http::Response<hyper::body::Incoming>> {
		let stream = tokio::net::UnixStream::connect(&self.path).await?;

		let (mut sender, connection) = hyper::client::conn::http1::handshake(stream)
			.await
			.map_err(Error::other)?;
		tokio::task::spawn(async move {
			connection.await.ok();
		});

		let response = sender.send_request(request).await.map_err(Error::other)?;

		Ok(response)
	}

	pub fn checkin(&self, path: &fs::Path) -> Result<artifact::Hash> {
		todo!()
	}
}
