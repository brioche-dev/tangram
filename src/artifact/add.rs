use super::{Artifact, Hash};
use crate::{
	blob,
	error::{Error, Result},
	Instance,
};
use lmdb::Transaction;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Outcome {
	Added { artifact_hash: Hash },
	DirectoryMissingEntries { entries: Vec<(String, Hash)> },
	FileMissingBlob { blob_hash: blob::Hash },
	ReferenceMissingArtifact { artifact_hash: Hash },
}

impl Instance {
	/// Add an artifact after ensuring all its references are present.
	pub async fn add_artifact(&self, artifact: &Artifact) -> Result<Hash> {
		match self.try_add_artifact(artifact).await? {
			Outcome::Added { artifact_hash } => Ok(artifact_hash),
			_ => Err(Error::message("Failed to add the artifact.")),
		}
	}

	/// Add an artifact after ensuring all its references are present.
	pub async fn try_add_artifact(&self, artifact: &Artifact) -> Result<Outcome> {
		match artifact {
			// If the artifact is a directory, then ensure all its entries are present.
			Artifact::Directory(directory) => {
				let mut entries = Vec::new();
				for (entry_name, artifact_hash) in &directory.entries {
					let artifact_hash = *artifact_hash;
					let exists = self.artifact_exists_local(artifact_hash)?;
					if !exists {
						entries.push((entry_name.clone(), artifact_hash));
					}
				}
				if !entries.is_empty() {
					return Ok(Outcome::DirectoryMissingEntries { entries });
				}
			},

			// If the artifact is a file, then ensure its blob is present.
			Artifact::File(file) => {
				let blob_path = self.blobs_path().join(file.blob_hash.to_string());
				let blob_exists = crate::util::fs::exists(&blob_path).await?;
				if !blob_exists {
					return Ok(Outcome::FileMissingBlob {
						blob_hash: file.blob_hash,
					});
				}
			},

			// If this artifact is a symlink, then there is nothing to ensure.
			Artifact::Symlink(_) => {},

			// If this artifact is a reference, then ensure the referenced artifact is present.
			Artifact::Reference(reference) => {
				let artifact_hash = reference.artifact_hash;
				let exists = self.artifact_exists_local(artifact_hash)?;
				if !exists {
					return Ok(Outcome::ReferenceMissingArtifact { artifact_hash });
				}
			},
		}

		// Hash the artifact.
		let artifact_hash = artifact.hash();

		// Serialize the artifact.
		let value = artifact.serialize_to_vec();

		// Begin a write transaction.
		let mut txn = self.database.env.begin_rw_txn()?;

		// Add the artifact to the database.
		match txn.put(
			self.database.artifacts,
			&artifact_hash.as_slice(),
			&value,
			lmdb::WriteFlags::NO_OVERWRITE,
		) {
			Ok(_) | Err(lmdb::Error::KeyExist) => Ok(()),
			Err(error) => Err(error),
		}?;

		// Commit the transaction.
		txn.commit()?;

		Ok(Outcome::Added { artifact_hash })
	}
}
