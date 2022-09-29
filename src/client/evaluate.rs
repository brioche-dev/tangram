use super::Client;
use crate::hash::Hash;
use anyhow::Result;

impl Client {
	pub async fn evaluate(&self, hash: Hash) -> Result<Hash> {
		let body = self
			.post(
				&format!("/expressions/{hash}/evaluate"),
				hyper::Body::empty(),
			)
			.await?;
		let body = hyper::body::to_bytes(body).await?;
		let output = String::from_utf8(body.to_vec())?.parse()?;
		Ok(output)
	}
}
