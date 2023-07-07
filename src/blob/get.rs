use super::Blob;
use crate::{
	error::{Result, WrapErr},
	instance::Instance,
};
use tokio::io::{AsyncRead, AsyncSeek};

impl Blob {
	pub async fn get(&self, tg: &Instance) -> Result<impl AsyncRead> {
		let reader = self
			.try_get(tg)
			.await?
			.wrap_err("Failed to find the blob.")?;
		Ok(reader)
	}

	pub async fn try_get(&self, tg: &Instance) -> Result<Option<impl AsyncRead>> {
		// Attempt to get the blob locally.
		if let Some(reader) = self.try_get_local(tg).await? {
			return Ok(Some(
				Box::new(reader) as Box<dyn AsyncRead + Send + Sync + Unpin>
			));
		}

		// Attempt to get the blob from the API.
		let reader = tg.api_client().try_get_blob(self.hash).await.ok().flatten();
		if let Some(reader) = reader {
			return Ok(Some(Box::new(reader)));
		}

		Ok(None)
	}

	pub async fn get_local(&self, tg: &Instance) -> Result<impl AsyncRead> {
		let reader = self
			.try_get_local(tg)
			.await?
			.wrap_err("Failed to find the blob.")?;
		Ok(reader)
	}

	pub async fn try_get_local(&self, tg: &Instance) -> Result<Option<impl AsyncRead + AsyncSeek>> {
		let path = tg.blob_path(self.hash);
		if !tokio::fs::try_exists(&path).await? {
			return Ok(None);
		}
		let file = tokio::fs::File::open(path)
			.await
			.wrap_err("Failed to open the blob file.")?;
		Ok(Some(file))
	}
}
