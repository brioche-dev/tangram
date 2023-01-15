use super::{Artifact, ArtifactHash};
use crate::{blob::BlobHash, util::path_exists, Cli};
use anyhow::{bail, Result};
use lmdb::Transaction;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum AddArtifactOutcome {
	Added {
		artifact_hash: ArtifactHash,
	},
	DirectoryMissingEntries {
		entries: Vec<(String, ArtifactHash)>,
	},
	FileMissingBlob {
		blob_hash: BlobHash,
	},
	DependencyMissing {
		artifact_hash: ArtifactHash,
	},
}

impl Cli {
	/// Add an artifact after ensuring all its references are present.
	pub async fn add_artifact(&self, artifact: &Artifact) -> Result<ArtifactHash> {
		match self.try_add_artifact(artifact).await? {
			AddArtifactOutcome::Added { artifact_hash } => Ok(artifact_hash),
			_ => bail!("Failed to add the artifact."),
		}
	}

	/// Add an artifact after ensuring all its references are present.
	pub async fn try_add_artifact(&self, artifact: &Artifact) -> Result<AddArtifactOutcome> {
		match artifact {
			// If the artifact is a directory, ensure all its entries are present.
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
					return Ok(AddArtifactOutcome::DirectoryMissingEntries { entries });
				}
			},

			// If the artifact is a file, ensure its blob is present.
			Artifact::File(file) => {
				let blob_path = self.blob_path(file.blob);
				let blob_exists = path_exists(&blob_path).await?;
				if !blob_exists {
					return Ok(AddArtifactOutcome::FileMissingBlob {
						blob_hash: file.blob,
					});
				}
			},

			// If this artifact is a symlink, there is nothing to ensure.
			Artifact::Symlink(_) => {},

			// If this artifact is a dependency, ensure its dependency artifact is present.
			Artifact::Dependency(dependency) => {
				let artifact_hash = dependency.artifact;
				let exists = self.artifact_exists_local(artifact_hash)?;
				if !exists {
					return Ok(AddArtifactOutcome::DependencyMissing { artifact_hash });
				}
			},
		}

		// Hash the artifact.
		let artifact_hash = artifact.hash();

		// Serialize the artifact.
		let value = artifact.serialize_to_vec();

		// Begin a write transaction.
		let mut txn = self.inner.database.env.begin_rw_txn()?;

		// Add the artifact to the database.
		match txn.put(
			self.inner.database.artifacts,
			&artifact_hash.as_slice(),
			&value,
			lmdb::WriteFlags::NO_OVERWRITE,
		) {
			Ok(_) | Err(lmdb::Error::KeyExist) => Ok(()),
			Err(error) => Err(error),
		}?;

		// Commit the transaction.
		txn.commit()?;

		Ok(AddArtifactOutcome::Added { artifact_hash })
	}
}
