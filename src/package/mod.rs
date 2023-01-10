#![allow(clippy::module_name_repetitions)]

pub use self::hash::PackageHash;
use crate::{
	artifact::ArtifactHash,
	hash::Hash,
	lock::Lock,
	lockfile::{self, Lockfile},
	manifest::Manifest,
	State,
};
use anyhow::{bail, Context, Result};
use async_recursion::async_recursion;
use byteorder::{ReadBytesExt, WriteBytesExt};
use camino::Utf8PathBuf;
use lmdb::Transaction;
use std::{collections::BTreeMap, path::Path};
use tokio::io::AsyncReadExt;

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
pub struct Package {
	#[buffalo(id = 0)]
	pub source: ArtifactHash,

	#[buffalo(id = 1)]
	pub dependencies: BTreeMap<String, PackageHash>,
}

impl Package {
	pub fn deserialize<R>(mut reader: R) -> Result<Package>
	where
		R: std::io::Read,
	{
		// Read the version.
		let version = reader.read_u8()?;
		if version != 0 {
			bail!(r#"Cannot deserialize package with version "{version}"."#);
		}

		// Deserialize the package.
		let package = buffalo::from_reader(reader)?;

		Ok(package)
	}

	pub fn serialize<W>(&self, mut writer: W) -> Result<()>
	where
		W: std::io::Write,
	{
		// Write the version.
		writer.write_u8(0)?;

		// Write the package.
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
	pub fn serialize_to_vec_and_hash(&self) -> (Vec<u8>, PackageHash) {
		let data = self.serialize_to_vec();
		let hash = PackageHash(Hash::new(&data));
		(data, hash)
	}

	#[must_use]
	pub fn hash(&self) -> PackageHash {
		let data = self.serialize_to_vec();
		PackageHash(Hash::new(data))
	}
}

impl State {
	pub fn package_exists_local(&self, package_hash: PackageHash) -> Result<bool> {
		// Begin a read transaction.
		let txn = self.database.env.begin_ro_txn()?;

		let exists = match txn.get(self.database.packages, &package_hash.as_slice()) {
			Ok(_) => Ok::<_, anyhow::Error>(true),
			Err(lmdb::Error::NotFound) => Ok(false),
			Err(error) => Err(error.into()),
		}?;

		Ok(exists)
	}
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum AddPackageOutcome {
	Added {
		package_hash: PackageHash,
	},
	MissingSource {
		source: ArtifactHash,
	},
	MissingDependencies {
		dependencies: Vec<(String, PackageHash)>,
	},
}

impl State {
	/// Add a pacakge after ensuring all its references are present.
	pub fn try_add_package(&self, package: &Package) -> Result<AddPackageOutcome> {
		// Ensure the package's source is present.
		let source = package.source;
		let exists = self.artifact_exists_local(source)?;
		if !exists {
			return Ok(AddPackageOutcome::MissingSource { source });
		}

		// Ensure all the package's dependencies are present.
		let mut dependencies = Vec::new();
		for (name, package_hash) in &package.dependencies {
			let package_hash = *package_hash;
			let exists = self.package_exists_local(package_hash)?;
			if !exists {
				dependencies.push((name.clone(), package_hash));
			}
		}
		if !dependencies.is_empty() {
			return Ok(AddPackageOutcome::MissingDependencies { dependencies });
		}

		// Hash the package.
		let package_hash = package.hash();

		// Serialize the package.
		let value = package.serialize_to_vec();

		// Begin a write transaction.
		let mut txn = self.database.env.begin_rw_txn()?;

		// Add the package to the database.
		match txn.put(
			self.database.packages,
			&package_hash.as_slice(),
			&value,
			lmdb::WriteFlags::NO_OVERWRITE,
		) {
			Ok(_) | Err(lmdb::Error::KeyExist) => Ok(()),
			Err(error) => Err(error),
		}?;

		// Commit the transaction.
		txn.commit()?;

		Ok(AddPackageOutcome::Added { package_hash })
	}
}

impl State {
	pub fn add_package(&self, package: &Package) -> Result<PackageHash> {
		match self.try_add_package(package)? {
			AddPackageOutcome::Added { package_hash } => Ok(package_hash),
			_ => bail!("Failed to add the package."),
		}
	}
}

impl State {
	/// Try to get a package from the database with the given transaction.
	pub fn try_get_package_local_with_txn<Txn>(
		&self,
		txn: &Txn,
		hash: PackageHash,
	) -> Result<Option<Package>>
	where
		Txn: lmdb::Transaction,
	{
		match txn.get(self.database.packages, &hash.as_slice()) {
			Ok(value) => {
				let value = Package::deserialize(value)?;
				Ok(Some(value))
			},
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => bail!(error),
		}
	}
}

impl State {
	pub fn get_package_local(&self, hash: PackageHash) -> Result<Package> {
		let package = self
			.try_get_package_local(hash)?
			.with_context(|| format!(r#"Failed to find the package with hash "{hash}"."#))?;
		Ok(package)
	}

	pub fn get_package_local_with_txn<Txn>(&self, txn: &Txn, hash: PackageHash) -> Result<Package>
	where
		Txn: lmdb::Transaction,
	{
		let package = self
			.try_get_package_local_with_txn(txn, hash)?
			.with_context(|| format!(r#"Failed to find the package with hash "{hash}"."#))?;
		Ok(package)
	}

	pub fn try_get_package_local(&self, hash: PackageHash) -> Result<Option<Package>> {
		// Begin a read transaction.
		let txn = self.database.env.begin_ro_txn()?;

		// Get the package.
		let maybe_package = self.try_get_package_local_with_txn(&txn, hash)?;

		Ok(maybe_package)
	}
}

impl State {
	/// Check in a package at the specified path.
	pub async fn checkin_package(&self, path: &Path, locked: bool) -> Result<PackageHash> {
		// Generate the lockfile if necessary.
		if !locked {
			self.generate_lockfile(path, locked)
				.await
				.with_context(|| {
					format!(
						r#"Failed to generate the lockfile for the package at path "{}"."#,
						path.display(),
					)
				})?;
		}

		// Check in the path.
		let package_source_hash = self
			.checkin(path)
			.await
			.context("Failed to check in the package.")?;

		// Read the lockfile.
		let lockfile_path = path.join("tangram.lock");
		let lockfile = tokio::fs::read(&lockfile_path)
			.await
			.context("Failed to read the lockfile.")?;
		let lockfile: Lockfile =
			serde_json::from_slice(&lockfile).context("Failed to deserialize the lockfile.")?;

		// Create the package.
		let dependencies = lockfile
			.as_v1()
			.context("Expected V1 Lockfile.")?
			.dependencies
			.iter()
			.map(|(name, entry)| (name.clone(), entry.hash))
			.collect();
		let package = Package {
			source: package_source_hash,
			dependencies,
		};

		// Add the package to the database.
		let package_hash = self.add_package(&package)?;

		Ok(package_hash)
	}

	#[async_recursion]
	#[must_use]
	pub async fn generate_lockfile(&self, path: &Path, locked: bool) -> Result<()> {
		// Open the manifest file.
		let manifest_path = path.join("tangram.json");
		let mut manifest_file = tokio::fs::File::open(&manifest_path)
			.await
			.with_context(|| {
				format!(
					r#"Failed to open the package manifest at path "{}"."#,
					manifest_path.display()
				)
			})?;

		// Acquire a lock on the manifest file to detect a cyclic dependency or concurrent lockfile generation.
		let manifest_lock = Lock::new(&manifest_path, ());
		let manifest_lock_result = manifest_lock
			.try_lock_exclusive()
			.await
			.context("Failed to acquire a lock on the manifest.")?;
		let _manifest_lock_guard = match manifest_lock_result {
			Some(guard) => guard,
			None => {
				bail!("Encountered a cyclic dependency or concurrent lockfile generation.")
			},
		};

		// Read the manifest.
		let mut manifest = String::new();
		manifest_file
			.read_to_string(&mut manifest)
			.await
			.context("Failed to read the package manifest.")?;

		// Deserialize the manifest.
		let manifest: Manifest = serde_json::from_str(&manifest).with_context(|| {
			format!(
				r#"Failed to deserialize the package manifest at path "{}"."#,
				manifest_path.display()
			)
		})?;

		// Get the dependencies.
		let mut dependencies = BTreeMap::new();
		for (dependency_name, dependency) in manifest.dependencies.unwrap_or_default() {
			// Retrieve the path dependency.
			let entry = match dependency {
				crate::manifest::Dependency::PathDependency(dependency) => {
					// Get the absolute path to the dependency.
					let dependency_path = path.join(&dependency.path);
					let dependency_path = tokio::fs::canonicalize(&dependency_path)
						.await
						.with_context(|| {
							format!("Could not canonicalize \"{}\"", dependency_path.display())
						})?;

					// Get the dependency's hash.
					let dependency_hash = self
						.checkin_package(&dependency_path, locked)
						.await
						.context("Failed to check in the dependency.")?;

					// Get the dependency package.
					let dependency_package = self.get_package_local(dependency_hash)?;

					// Create the lockfile entry.
					lockfile::Dependency {
						hash: dependency_hash,
						source: dependency_package.source,
						dependencies: None,
					}
				},

				// Handle a registry dependency.
				crate::manifest::Dependency::RegistryDependency(_) => {
					todo!()

					// // Get the package hash from the registry.
					// let dependency_version = &dependency.version;
					// let dependency_hash = self.api_client
					// 	.get_package_version(&dependency_name, &dependency.version)
					// 	.await
					// 	.with_context(||
					// 		format!(r#"Package with name "{dependency_name}" and version "{dependency_version}" is not in the package registry."#)
					// 	)?;
					// let dependency_source_hash = self.get_package_source(dependency_hash)?;

					// // Create the lockfile Entry.
					// lockfile::Dependency {
					// 	hash: dependency_hash,
					// 	source: dependency_source_hash,
					// 	dependencies: None,
					// }
				},
			};

			// Add the dependency.
			dependencies.insert(dependency_name.clone(), entry);
		}

		// Create and write the lockfile.
		let lockfile = Lockfile::new_v1(dependencies);
		let lockfile =
			serde_json::to_vec_pretty(&lockfile).context("Failed to serialize the lockfile.")?;
		let lockfile_path = path.join("tangram.lock");
		tokio::fs::write(&lockfile_path, lockfile)
			.await
			.context("Failed to write the lockfile.")?;

		Ok(())
	}

	pub fn get_package_source(&self, package_hash: PackageHash) -> Result<ArtifactHash> {
		let package = self.get_package_local(package_hash)?;
		let package_source = package.source;
		Ok(package_source)
	}

	/// Get the manifest for the package with the given package hash.
	pub async fn get_package_manifest(&self, package_hash: PackageHash) -> Result<Manifest> {
		let package_source_artifact_hash = self.get_package_source(package_hash)?;

		// Get the source directory.
		let source_directory = self
			.get_artifact_local(package_source_artifact_hash)?
			.into_directory()
			.context("Expected a directory.")?;

		// Get the manifest artifact hash.
		let manifest_artifact_hash = source_directory
			.entries
			.get("tangram.json")
			.copied()
			.context("The package source does not contain a manifest.")?;

		// Get the manifest blob hash.
		let manifest_blob_hash = self
			.get_artifact_local(manifest_artifact_hash)?
			.as_file()
			.context("Expected the manifest to be a file.")?
			.blob;

		// Read the manifest.
		let mut manifest = self
			.get_blob(manifest_blob_hash)
			.await
			.context("Failed to get the manifest blob.")?;
		let mut manifest_bytes = Vec::new();
		manifest
			.read_to_end(&mut manifest_bytes)
			.await
			.context("Failed to read the manifest.")?;

		// Deserialize the manifest.
		let manifest: Manifest = serde_json::from_slice(&manifest_bytes)
			.context(r#"Failed to parse the package manifest."#)?;

		Ok(manifest)
	}
}

impl State {
	pub fn get_package_entrypoint_path(
		&self,
		package_hash: PackageHash,
	) -> Result<Option<Utf8PathBuf>> {
		const ENTRYPOINT_FILE_NAMES: [&str; 2] = ["tangram.ts", "tangram.js"];

		// Get the package source directory.
		let source_hash = self
			.get_package_source(package_hash)
			.context("Failed to get the package source.")?;
		let source_directory = self
			.get_artifact_local(source_hash)
			.context("Failed to get the package source.")?
			.into_directory()
			.context("The package source must be a directory.")?;

		// Get the entrypoint.
		let entrypoint = ENTRYPOINT_FILE_NAMES
			.into_iter()
			.find(|file_name| source_directory.entries.contains_key(*file_name))
			.map(Utf8PathBuf::from);

		Ok(entrypoint)
	}
}
