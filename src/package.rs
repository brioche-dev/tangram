use crate::{
	builder,
	expression::Expression,
	hash::Hash,
	lockfile::{self, Lockfile},
	manifest::Manifest,
};
use anyhow::{anyhow, Context, Result};
use fnv::FnvBuildHasher;
use std::{
	collections::{BTreeMap, HashMap, VecDeque},
	path::{Path, PathBuf},
};

impl builder::Shared {
	/// Checkin a package from the provided source path.
	#[allow(clippy::too_many_lines)]
	pub async fn checkin_package(&self, source_path: &Path, locked: bool) -> Result<Hash> {
		let source_path = tokio::fs::canonicalize(source_path).await?;

		// Collect all path dependencies in topological order.
		let mut queue: VecDeque<PathBuf> = VecDeque::from(vec![source_path.clone()]);
		let mut package_source_paths: Vec<PathBuf> = Vec::new();
		while let Some(package_path) = queue.pop_front() {
			// Add the path to the list of package paths.
			package_source_paths.push(package_path.clone());

			// Read the manifest.
			let manifest_path = package_path.join("tangram.json");
			let manifest = tokio::fs::read(&manifest_path)
				.await
				.context("Failed to read the package manifest.")?;
			let manifest: Manifest = serde_json::from_slice(&manifest).with_context(|| {
				format!(
					r#"Failed to parse the package manifest at path "{}"."#,
					manifest_path.display()
				)
			})?;

			// Add the package's path dependencies to the queue.
			if let Some(dependencies) = manifest.dependencies {
				for dependency in dependencies.values() {
					match dependency {
						crate::manifest::Dependency::PathDependency(dependency) => {
							let dependency_path = package_path.join(&dependency.path);
							let dependency_path = tokio::fs::canonicalize(&dependency_path)
								.await
								.with_context(|| {
								format!(
									r#"Failed to canonicalize the dependency at path "{}"."#,
									dependency_path.display()
								)
							})?;
							queue.push_back(dependency_path);
						},
						crate::manifest::Dependency::RegistryDependency(_) => continue,
					}
				}
			}
		}

		// Reverse the package source paths to put them in reverse topological order.
		package_source_paths.reverse();

		// Write the lockfile for each package source, check it in, and create its package expression.
		let mut cache: HashMap<PathBuf, Hash, FnvBuildHasher> = HashMap::default();
		let mut root_package = None;
		for package_source_path in package_source_paths {
			// If this package has already been checked in, then continue.
			if cache.get(&package_source_path).is_some() {
				continue;
			}

			// Read the manifest.
			let manifest_path = package_source_path.join("tangram.json");
			let manifest = tokio::fs::read(&manifest_path).await?;
			let manifest: Manifest = serde_json::from_slice(&manifest)?;

			if !locked {
				// Create the lockfile for this package.
				let mut dependencies = BTreeMap::new();
				for (dependency_name, dependency) in manifest.dependencies.iter().flatten() {
					// Retrieve the path dependency.
					let entry = match dependency {
						crate::manifest::Dependency::PathDependency(dependency) => {
							// Get the absolute path to the dependency.
							let dependency_path = package_source_path.join(&dependency.path);
							let dependency_path = tokio::fs::canonicalize(&dependency_path).await?;

							// Get the dependency's expression hash.
							let dependency_hash =
								cache.get(&dependency_path).copied().ok_or_else(|| {
									anyhow!(
										r#"Failed to get the artifact for path "{}"."#,
										dependency_path.display(),
									)
								})?;

							// Get the dependency's source.
							let dependency_source =
								self.get_package_source(dependency_hash).await?;

							// Create the lockfile entry.
							lockfile::Dependency {
								hash: dependency_hash,
								source: dependency_source,
								dependencies: None,
							}
						},

						// Handle a registry dependency.
						crate::manifest::Dependency::RegistryDependency(dependency) => {
							// Get the package hash from the registry.
							let dependency_version = &dependency.version;
							let package_hash = self
								.get_package_version(dependency_name, &dependency.version)
								.await?
								.ok_or_else(|| anyhow!(
									r#"Package with name "{dependency_name}" and version "{dependency_version}" is not in the package registry."#
								))?;
							let package_source_hash = self.get_package_source(package_hash).await?;

							// Create the lockfile Entry.
							lockfile::Dependency {
								hash: package_hash,
								source: package_source_hash,
								dependencies: None,
							}
						},
					};

					// Add the dependency.
					dependencies.insert(dependency_name.clone(), entry);
				}

				// Write the lockfile.
				let lockfile = Lockfile::new_v1(dependencies);
				let lockfile = serde_json::to_vec_pretty(&lockfile)?;
				let lockfile_path = package_source_path.join("tangram.lock");
				tokio::fs::write(&lockfile_path, lockfile).await?;
			};

			// Check in the package source.
			let package_source_hash = self.checkin(&package_source_path).await?;

			// Read the lockfile.
			let lockfile_path = package_source_path.join("tangram.lock");
			let lockfile = tokio::fs::read(&lockfile_path).await?;
			let lockfile: Lockfile = serde_json::from_slice(&lockfile)?;

			// Create the package expression.
			let dependencies = lockfile
				.as_v1()
				.ok_or_else(|| anyhow!("Expected V1 Lockfile."))?
				.dependencies
				.iter()
				.map(|(name, entry)| (name.clone().into(), entry.hash))
				.collect();
			let dependencies = self.add_expression(&Expression::Map(dependencies)).await?;

			let mut package = BTreeMap::new();
			package.insert("dependencies".into(), dependencies);
			package.insert("source".into(), package_source_hash);
			let package_hash = self.add_expression(&Expression::Map(package)).await?;

			// Add the package to the cache.
			cache.insert(package_source_path.clone(), package_hash);

			root_package = Some(package_hash);
		}

		let root_package = root_package.unwrap();
		Ok(root_package)
	}

	pub async fn get_package_source(&self, package_hash: Hash) -> Result<Hash> {
		let package_map = self
			.get_expression(package_hash)
			.await?
			.into_map()
			.ok_or_else(|| anyhow!("Expected map."))?;
		let package_source = package_map
			.get("source")
			.copied()
			.ok_or_else(|| anyhow!("Expected source."))?;
		Ok(package_source)
	}

	pub async fn get_package_manifest(&self, package_hash: Hash) -> Result<Manifest> {
		let package_source_hash = self.get_package_source(package_hash).await?;

		let source_artifact = self
			.get_expression(package_source_hash)
			.await?
			.into_artifact()
			.ok_or_else(|| anyhow!("Expected an artifact."))?;

		let source_directory = self
			.get_expression(source_artifact.root)
			.await?
			.into_directory()
			.ok_or_else(|| anyhow!("Expected a directory."))?;

		let manifest_hash = source_directory
			.entries
			.get("tangram.json")
			.copied()
			.ok_or_else(|| anyhow!("The package source does not contain a manifest."))?;

		let manifest_blob_hash = self
			.get_expression(manifest_hash)
			.await?
			.as_file()
			.ok_or_else(|| anyhow!("Expected the manifest to be a file."))?
			.blob;

		let manifest_path = self.get_blob(manifest_blob_hash).await?;

		let manifest = tokio::fs::read(&manifest_path)
			.await
			.context("Failed to read the package manifest.")?;

		let manifest: Manifest = serde_json::from_slice(&manifest)
			.context(r#"Failed to parse the package manifest."#)?;

		Ok(manifest)
	}
}

#[derive(serde::Serialize)]
pub struct Version {
	version: String,
	artifact: Hash,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
	pub name: String,
}

impl builder::Shared {
	pub async fn search_packages(&self, name: &str) -> Result<Vec<SearchResult>> {
		// Retrieve packages that match this query.
		let packages = self
			.database_transaction(|txn| {
				let sql = r#"
					select
						name
					from
						packages
					where
						name like ?1
				"#;
				let params = (format!("%{name}%"),);
				let mut statement = txn
					.prepare_cached(sql)
					.context("Failed to prepare the query.")?;
				let items = statement
					.query(params)
					.context("Failed to exeucte the query.")?
					.and_then(|row| {
						let name = row.get::<_, String>(0)?;
						let item = SearchResult { name };
						Ok(item)
					})
					.collect::<Result<_>>()?;
				Ok(items)
			})
			.await?;
		Ok(packages)
	}

	pub async fn get_packages(&self) -> Result<Vec<SearchResult>> {
		// Retrieve packages that match this query.
		let packages = self
			.database_transaction(|txn| {
				let sql = r#"
					select
						name
					from
						packages
				"#;
				let mut statement = txn
					.prepare_cached(sql)
					.context("Failed to prepare the query.")?;
				let items = statement
					.query(())
					.context("Failed to execute the query.")?
					.and_then(|row| {
						let name = row.get::<_, String>(0)?;
						let item = SearchResult { name };
						Ok(item)
					})
					.collect::<Result<_>>()?;
				Ok(items)
			})
			.await?;
		Ok(packages)
	}

	pub async fn get_package(&self, package_name: &str) -> Result<Vec<Version>> {
		// Retrieve the package versions.
		let versions = self
			.database_transaction(|txn| {
				let sql = r#"
					select
						version,
						hash
					from
						package_versions
					where
						name = ?1
				"#;
				let params = (package_name,);
				let mut statement = txn
					.prepare_cached(sql)
					.context("Failed to prepare the query.")?;
				let versions = statement
					.query(params)
					.context("Failed to execute the query.")?
					.and_then(|row| {
						let version = row.get::<_, String>(0)?;
						let hash = row.get::<_, String>(1)?;
						let hash = hash.parse().with_context(|| "Failed to parse the hash.")?;
						let package_version = Version {
							version,
							artifact: hash,
						};
						Ok(package_version)
					})
					.collect::<Result<_>>()?;
				Ok(versions)
			})
			.await?;

		Ok(versions)
	}

	pub async fn create_package(&self, package_name: &str) -> Result<()> {
		self.database_transaction(|txn| {
			// Check if the package already exists.
			let sql = r#"
				select
					count(*) > 0
				from
					packages
				where
					name = ?1
			"#;
			let params = (package_name,);
			let mut statement = txn
				.prepare_cached(sql)
				.context("Failed to prepare the query.")?;
			let package_exists = statement
				.query(params)
				.context("Failed to execute the query.")?
				.and_then(|row| row.get::<_, bool>(0))
				.next()
				.transpose()?
				.unwrap();

			if !package_exists {
				// Create the package.
				let sql = r#"
					insert into packages (
						name
					) values (
						?1
					)
				"#;
				let params = (package_name,);
				let mut statement = txn
					.prepare_cached(sql)
					.context("Failed to prepare the query.")?;
				statement
					.execute(params)
					.context("Failed to execute the query.")?;
			}

			Ok(())
		})
		.await?;

		Ok(())
	}
}

impl builder::Shared {
	// Retrieve the artifact for a given package name and version.
	pub async fn get_package_version(
		&self,
		package_name: &str,
		package_version: &str,
	) -> Result<Option<Hash>> {
		// Retrieve the artifact hash from the database.
		self.database_transaction(|txn| {
			let sql = r#"
				select
					hash
				from
					package_versions
				where
					name = ?1 and version = ?2
			"#;
			let params = (package_name, package_version.to_string());
			let mut statement = txn
				.prepare_cached(sql)
				.context("Failed to prepare the query.")?;
			let maybe_hash = statement
				.query(params)
				.context("Failed to execute the query.")?
				.and_then(|row| row.get::<_, String>(0))
				.next()
				.transpose()?;
			let maybe_hash = if let Some(hash) = maybe_hash {
				let hash = hash.parse().context("Failed to parse the hash.")?;
				Some(hash)
			} else {
				None
			};
			Ok(maybe_hash)
		})
		.await
	}

	// Create a new package version given an artifact.
	pub async fn create_package_version(
		&self,
		package_name: &str,
		package_version: &str,
		artifact: Hash,
	) -> Result<Hash> {
		self.database_transaction(|txn| {
			// Check if the package already exists.
			let sql = r#"
				select
					count(*) > 0
				from
					packages
				where
					name = ?1
			"#;
			let params = (package_name,);
			let mut statement = txn
				.prepare_cached(sql)
				.context("Failed to prepare the query.")?;
			let package_exists = statement
				.query(params)
				.context("Failed to execute the query.")?
				.and_then(|row| row.get::<_, bool>(0))
				.next()
				.transpose()?
				.unwrap();

			// Create the package if it does not exist.
			if !package_exists {
				let sql = r#"
					insert into packages (
						name
					) values (
						?1
					)
				"#;
				let params = (package_name,);
				let mut statement = txn
					.prepare_cached(sql)
					.context("Failed to prepare the query.")?;
				statement
					.execute(params)
					.context("Failed to execute the query.")?;
			}

			// Check if the package version already exists.
			let sql = r#"
				select
					count(*) > 0
				from
					package_versions
				where
					name = ?1 and version = ?2
			"#;
			let params = (package_name, package_version);
			let mut statement = txn
				.prepare_cached(sql)
				.context("Failed to prepare the query.")?;
			let package_version_exists = statement
				.query(params)
				.context("Failed to execute the query.")?
				.and_then(|row| row.get::<_, bool>(0))
				.next()
				.transpose()?
				.unwrap();

			if package_version_exists {
				return Err(anyhow!(format!(
					r#"The package with name "{package_name}" and version "{package_version}" already exists."#
				)));
			}

			// Create the new package version.
			let sql = r#"
				insert into package_versions (
					name, version, hash
				) values (
					?1, ?2, ?3
				)
			"#;
			let params = (package_name, package_version, artifact.to_string());
			let mut statement = txn
				.prepare_cached(sql)
				.context("Failed to prepare the query.")?;
			statement
				.execute(params)
				.context("Failed to execute the query.")?;
			drop(statement);

			Ok(())
		})
		.await?;

		Ok(artifact)
	}
}
