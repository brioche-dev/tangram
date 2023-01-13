pub use self::{
	dependency::Dependency, directory::Directory, file::File, hash::ArtifactHash, symlink::Symlink,
};
use crate::{blob::BlobHash, util::path_exists, Cli};
use anyhow::{bail, Context, Result};
use lmdb::Transaction;

mod dependency;
mod directory;
mod file;
mod hash;
mod serialize;
mod symlink;

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
#[serde(tag = "type", content = "value")]
pub enum Artifact {
	#[buffalo(id = 0)]
	#[serde(rename = "directory")]
	Directory(Directory),

	#[buffalo(id = 1)]
	#[serde(rename = "file")]
	File(File),

	#[buffalo(id = 2)]
	#[serde(rename = "symlink")]
	Symlink(Symlink),

	#[buffalo(id = 3)]
	#[serde(rename = "dependency")]
	Dependency(Dependency),
}

impl Artifact {
	#[must_use]
	pub fn as_directory(&self) -> Option<&Directory> {
		if let Artifact::Directory(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_file(&self) -> Option<&File> {
		if let Artifact::File(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_symlink(&self) -> Option<&Symlink> {
		if let Artifact::Symlink(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_dependency(&self) -> Option<&Dependency> {
		if let Artifact::Dependency(v) = self {
			Some(v)
		} else {
			None
		}
	}
}

impl Artifact {
	#[must_use]
	pub fn into_directory(self) -> Option<Directory> {
		if let Artifact::Directory(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_file(self) -> Option<File> {
		if let Artifact::File(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_symlink(self) -> Option<Symlink> {
		if let Artifact::Symlink(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_dependency(self) -> Option<Dependency> {
		if let Artifact::Dependency(v) = self {
			Some(v)
		} else {
			None
		}
	}
}

impl Cli {
	pub fn artifact_exists_local(&self, artifact_hash: ArtifactHash) -> Result<bool> {
		// Begin a read transaction.
		let txn = self.state.database.env.begin_ro_txn()?;

		let exists = match txn.get(self.state.database.artifacts, &artifact_hash.as_slice()) {
			Ok(_) => Ok::<_, anyhow::Error>(true),
			Err(lmdb::Error::NotFound) => Ok(false),
			Err(error) => Err(error.into()),
		}?;

		Ok(exists)
	}
}

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
		let mut txn = self.state.database.env.begin_rw_txn()?;

		// Add the artifact to the database.
		match txn.put(
			self.state.database.artifacts,
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

impl Cli {
	pub async fn add_artifact(&self, artifact: &Artifact) -> Result<ArtifactHash> {
		match self.try_add_artifact(artifact).await? {
			AddArtifactOutcome::Added { artifact_hash } => Ok(artifact_hash),
			_ => bail!("Failed to add the artifact."),
		}
	}
}

impl Cli {
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
		let txn = self.state.database.env.begin_ro_txn()?;

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
		match txn.get(self.state.database.artifacts, &hash.as_slice()) {
			Ok(value) => {
				let value = Artifact::deserialize(value)?;
				Ok(Some(value))
			},
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => bail!(error),
		}
	}
}
