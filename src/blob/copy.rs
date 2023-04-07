use super::Blob;
use crate::{error::Result, instance::Instance, util::fs};
use tokio::io::AsyncWrite;

impl Blob {
	pub async fn copy_to_path(&self, tg: &Instance, path: &fs::Path) -> Result<()> {
		tokio::fs::copy(self.path(tg), path).await?;
		Ok(())
	}

	pub async fn copy_to_writer<W>(&self, tg: &Instance, writer: &mut W) -> Result<()>
	where
		W: AsyncWrite + Unpin,
	{
		let mut file = tokio::fs::File::open(self.path(tg)).await?;
		tokio::io::copy(&mut file, writer).await?;
		Ok(())
	}
}
