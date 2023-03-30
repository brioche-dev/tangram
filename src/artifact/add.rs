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
	MissingEntries { entries: Vec<(String, Hash)> },
	MissingBlob { blob_hash: blob::Hash },
	MissingReferences { artifact_hashes: Vec<Hash> },
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
					return Ok(Outcome::MissingEntries { entries });
				}
			},

			// If the artifact is a file, then ensure its blob is present and its references are present.
			Artifact::File(file) => {
				// Ensure the blob is present.
				let blob_path = self.blob_path(file.blob_hash);
				let blob_exists = crate::util::fs::exists(&blob_path).await?;
				if !blob_exists {
					return Ok(Outcome::MissingBlob {
						blob_hash: file.blob_hash,
					});
				}

				// Ensure the references are present.
				let mut missing_references = Vec::new();
				for artifact_hash in &file.references {
					let artifact_hash = *artifact_hash;
					let exists = self.artifact_exists_local(artifact_hash)?;
					if !exists {
						missing_references.push(artifact_hash);
					}
				}
				if !missing_references.is_empty() {
					return Ok(Outcome::MissingReferences {
						artifact_hashes: missing_references,
					});
				}
			},

			// If this artifact is a symlink, then ensure the artifacts referenced by its template are present.
			Artifact::Symlink(symlink) => {
				let mut missing_references = Vec::new();
				for component in &symlink.target.components {
					if let crate::template::Component::Artifact(artifact_hash) = component {
						let artifact_hash = *artifact_hash;
						let exists = self.artifact_exists_local(artifact_hash)?;
						if !exists {
							missing_references.push(artifact_hash);
						}
					}
				}
				if !missing_references.is_empty() {
					return Ok(Outcome::MissingReferences {
						artifact_hashes: missing_references,
					});
				}
			},
		}

		// Serialize and hash the artifact.
		let (value, hash) = artifact.serialize_to_vec_and_hash();

		// Begin a write transaction.
		let mut txn = self.database.env.begin_rw_txn()?;

		// Add the artifact to the database.
		match txn.put(
			self.database.artifacts,
			&hash.as_slice(),
			&value,
			lmdb::WriteFlags::NO_OVERWRITE,
		) {
			Ok(_) | Err(lmdb::Error::KeyExist) => Ok(()),
			Err(error) => Err(error),
		}?;

		// Commit the transaction.
		txn.commit()?;

		Ok(Outcome::Added {
			artifact_hash: hash,
		})
	}
}
