use super::{Artifact, ArtifactHash};
use crate::Cli;
use anyhow::{bail, Context, Result};
use lmdb::Transaction;

impl Cli {
	pub fn artifact_exists_local(&self, artifact_hash: ArtifactHash) -> Result<bool> {
		// Begin a read transaction.
		let txn = self.inner.database.env.begin_ro_txn()?;

		let exists = match txn.get(self.inner.database.artifacts, &artifact_hash.as_slice()) {
			Ok(_) => Ok::<_, anyhow::Error>(true),
			Err(lmdb::Error::NotFound) => Ok(false),
			Err(error) => Err(error.into()),
		}?;

		Ok(exists)
	}

	pub fn get_artifact_local(&self, hash: ArtifactHash) -> Result<Artifact> {
		let artifact = self
			.try_get_artifact_local(hash)?
			.with_context(|| format!(r#"Failed to find the artifact with hash "{hash}"."#))?;
		Ok(artifact)
	}

	pub fn get_artifact_local_with_txn<Txn>(
		&self,
		txn: &Txn,
		hash: ArtifactHash,
	) -> Result<Artifact>
	where
		Txn: lmdb::Transaction,
	{
		let artifact = self
			.try_get_artifact_local_with_txn(txn, hash)?
			.with_context(|| format!(r#"Failed to find the artifact with hash "{hash}"."#))?;
		Ok(artifact)
	}

	pub fn try_get_artifact_local(&self, hash: ArtifactHash) -> Result<Option<Artifact>> {
		// Begin a read transaction.
		let txn = self.inner.database.env.begin_ro_txn()?;

		// Get the artifact.
		let maybe_artifact = self.try_get_artifact_local_with_txn(&txn, hash)?;

		Ok(maybe_artifact)
	}

	/// Try to get an artifact from the database with the given transaction.
	pub fn try_get_artifact_local_with_txn<Txn>(
		&self,
		txn: &Txn,
		hash: ArtifactHash,
	) -> Result<Option<Artifact>>
	where
		Txn: lmdb::Transaction,
	{
		match txn.get(self.inner.database.artifacts, &hash.as_slice()) {
			Ok(value) => {
				let value = Artifact::deserialize(value)?;
				Ok(Some(value))
			},
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => bail!(error),
		}
	}
}
