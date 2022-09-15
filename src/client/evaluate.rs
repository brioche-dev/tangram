use super::Client;
use crate::hash::Hash;
use anyhow::Result;

impl Client {
	pub async fn evaluate(&self, hash: Hash) -> Result<Hash> {
		match self.transport.as_in_process_or_http() {
			super::transport::InProcessOrHttp::InProcess(server) => {
				let output = server.evaluate(hash, hash).await?;
				Ok(output)
			},
			super::transport::InProcessOrHttp::Http(http) => {
				let body = http
					.post(
						&format!("/expressions/{hash}/evaluate"),
						hyper::Body::empty(),
					)
					.await?;
				let body = hyper::body::to_bytes(body).await?;
				let output = String::from_utf8(body.to_vec())?.parse()?;
				Ok(output)
			},
		}
	}
}
