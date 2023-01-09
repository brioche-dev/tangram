#![allow(clippy::module_name_repetitions)]

pub use self::hash::ArtifactHash;
use crate::{blob::BlobHash, hash::Hash, util::path_exists, State};
use anyhow::{bail, Context, Result};
use byteorder::{ReadBytesExt, WriteBytesExt};
use camino::Utf8PathBuf;
use lmdb::Transaction;
use std::collections::BTreeMap;

mod hash;

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
pub struct Directory {
	#[buffalo(id = 0)]
	pub entries: BTreeMap<String, ArtifactHash>,
}

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
pub struct File {
	#[buffalo(id = 0)]
	pub blob: BlobHash,

	#[buffalo(id = 1)]
	pub executable: bool,
}

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
pub struct Symlink {
	#[buffalo(id = 0)]
	pub target: Utf8PathBuf,
}

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
pub struct Dependency {
	#[buffalo(id = 0)]
	pub artifact: ArtifactHash,

	#[buffalo(id = 1)]
	pub path: Option<Utf8PathBuf>,
}

impl Artifact {
	pub fn deserialize<R>(mut reader: R) -> Result<Artifact>
	where
		R: std::io::Read,
	{
		// Read the version.
		let version = reader.read_u8()?;
		if version != 0 {
			bail!(r#"Cannot deserialize artifact with version "{version}"."#);
		}

		// Deserialize the artifact.
		let artifact = buffalo::from_reader(reader)?;

		Ok(artifact)
	}

	pub fn serialize<W>(&self, mut writer: W) -> Result<()>
	where
		W: std::io::Write,
	{
		// Write the version.
		writer.write_u8(0)?;

		// Write the artifact.
		buffalo::to_writer(self, &mut writer)?;

		Ok(())
	}

	#[must_use]
	pub fn serialize_to_vec(&self) -> Vec<u8> {
		let mut data = Vec::new();
		self.serialize(&mut data).unwrap();
		data
	}

	#[must_use]
	pub fn serialize_to_vec_and_hash(&self) -> (Vec<u8>, ArtifactHash) {
		let data = self.serialize_to_vec();
		let hash = ArtifactHash(Hash::new(&data));
		(data, hash)
	}

	#[must_use]
	pub fn hash(&self) -> ArtifactHash {
		let data = self.serialize_to_vec();
		ArtifactHash(Hash::new(data))
	}
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

impl State {
	pub fn artifact_exists_local(&self, artifact_hash: ArtifactHash) -> Result<bool> {
		// Begin a read transaction.
		let txn = self.database.env.begin_ro_txn()?;

		let exists = match txn.get(self.database.artifacts, &artifact_hash.as_slice()) {
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

impl State {
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
		let value = buffalo::to_vec(artifact).unwrap();

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

		Ok(AddArtifactOutcome::Added { artifact_hash })
	}
}

impl State {
	pub async fn add_artifact(&self, artifact: &Artifact) -> Result<ArtifactHash> {
		match self.try_add_artifact(artifact).await? {
			AddArtifactOutcome::Added { artifact_hash } => Ok(artifact_hash),
			_ => bail!("Failed to add the artifact."),
		}
	}
}

impl State {
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
		let txn = self.database.env.begin_ro_txn()?;

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
		match txn.get(self.database.artifacts, &hash.as_slice()) {
			Ok(value) => {
				let value = buffalo::from_slice(value)?;
				Ok(Some(value))
			},
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => bail!(error),
		}
	}
}
