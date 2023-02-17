use super::{Algorithm, Checksum, Writer};
use crate::{artifact, Cli};
use anyhow::Result;
use async_recursion::async_recursion;

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn compute_artifact_checksum(
		&self,
		artifact_hash: artifact::Hash,
		algorithm: Algorithm,
	) -> Result<Checksum> {
		let mut writer = Writer::new(algorithm);
		self.compute_artifact_checksum_inner(artifact_hash, &mut writer)
			.await?;
		let checksum = writer.finalize();
		Ok(checksum)
	}

	#[async_recursion]
	pub async fn compute_artifact_checksum_inner(
		&self,
		_artifact_hash: artifact::Hash,
		_writer: &mut Writer,
	) -> Result<()> {
		todo!()
	}
}
