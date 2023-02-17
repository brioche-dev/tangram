use super::{Artifact, Hash};
use crate::Cli;
use anyhow::{bail, Context, Result};
use lmdb::Transaction;

impl Cli {
	pub fn artifact_exists_local(&self, artifact_hash: Hash) -> Result<bool> {
		// Begin a read transaction.
		let txn = self.database.env.begin_ro_txn()?;

		let exists = match txn.get(self.database.artifacts, &artifact_hash.as_slice()) {
			Ok(_) => Ok::<_, anyhow::Error>(true),
			Err(lmdb::Error::NotFound) => Ok(false),
			Err(error) => Err(error.into()),
		}?;

		Ok(exists)
	}

	pub fn get_artifact_local(&self, artifact_hash: Hash) -> Result<Artifact> {
		let artifact = self
			.try_get_artifact_local(artifact_hash)?
			.with_context(|| {
				format!(r#"Failed to find the artifact with hash "{artifact_hash}"."#)
			})?;
		Ok(artifact)
	}

	pub fn get_artifact_local_with_txn<Txn>(
		&self,
		txn: &Txn,
		artifact_hash: Hash,
	) -> Result<Artifact>
	where
		Txn: lmdb::Transaction,
	{
		let artifact = self
			.try_get_artifact_local_with_txn(txn, artifact_hash)?
			.with_context(|| {
				format!(r#"Failed to find the artifact with hash "{artifact_hash}"."#)
			})?;
		Ok(artifact)
	}

	pub fn try_get_artifact_local(&self, hash: Hash) -> Result<Option<Artifact>> {
		// Begin a read transaction.
		let txn = self.database.env.begin_ro_txn()?;

		// Get the artifact.
		let maybe_artifact = self.try_get_artifact_local_with_txn(&txn, hash)?;

		Ok(maybe_artifact)
	}

	/// Try to get an artifact from the database with the given transaction.
	pub fn try_get_artifact_local_with_txn<Txn>(
		&self,
		txn: &Txn,
		hash: Hash,
	) -> Result<Option<Artifact>>
	where
		Txn: lmdb::Transaction,
	{
		match txn.get(self.database.artifacts, &hash.as_slice()) {
			Ok(value) => {
				let value = Artifact::deserialize(value)?;
				Ok(Some(value))
			},
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => bail!(error),
		}
	}
}
