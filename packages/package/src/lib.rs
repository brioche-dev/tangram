use async_recursion::async_recursion;
use async_trait::async_trait;
use futures::stream::{FuturesUnordered, TryStreamExt};
use im::HashMap;
use itertools::Itertools;
use std::{
	collections::{BTreeMap, BTreeSet, HashSet, VecDeque},
	path::{Path, PathBuf},
};
use tangram_client as tg;
use tangram_error::{return_error, error, Result, WrapErr};
use tg::Client;
use tg::{dependency, Dependency};

/// The file name of the root module in a package.
pub const ROOT_MODULE_FILE_NAME: &str = "tangram.tg";

/// The file name of the lockfile in a package.
pub const LOCKFILE_FILE_NAME: &str = "tangram.lock";

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Lockfile {
	pub locks: Vec<Lock>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub struct Lock {
	pub dependencies: BTreeMap<tg::Dependency, Entry>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize)]
pub struct Entry {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub package: Option<tg::directory::Id>,
	pub lock: usize,
}

#[derive(Clone, Debug)]
struct PackageWithPathDependencies {
	pub package: tg::Directory,
	pub path_dependencies: BTreeMap<tg::Dependency, PackageWithPathDependencies>,
}

#[async_trait]
pub trait Ext {
	async fn metadata(&self, client: &dyn tg::Client) -> Result<tg::package::Metadata>;
	async fn dependencies(&self, client: &dyn tg::Client) -> Result<Vec<tg::Dependency>>;
}

struct Context {
	// The client.
	client: Box<dyn tg::Client>,

	// A cache of package analysis (metadata, direct dependencies).
	analysis: BTreeMap<tg::directory::Id, Analysis>,

	// A cache of published packages that we know about.
	published_packages: im::HashMap<tg::package::Metadata, tg::directory::Id>,

	// A table of path dependencies.
	path_dependencies: BTreeMap<tg::directory::Id, BTreeMap<tg::Path, tg::directory::Id>>,
}

/// An error type that can be pretty printed to describe why version solving failed.
struct Report {
	errors: Vec<(Dependant, Error)>,
	context: Context,
	solution: Solution,
}

#[derive(Clone, Debug)]
struct Frame {
	solution: Solution,
	working_set: im::Vector<Dependant>,
	remaining_versions: Option<im::Vector<String>>,
	last_error: Option<Error>,
}

#[derive(Clone, Debug, Default)]
struct Solution {
	permanent: im::HashMap<String, Result<tg::directory::Id, Error>>,
	partial: im::HashMap<Dependant, Mark>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Dependant {
	package: tg::directory::Id,
	dependency: tg::Dependency,
}

#[derive(Clone, Debug)]
enum Mark {
	Temporary(tg::directory::Id),
	Permanent(Result<tg::directory::Id, Error>),
}

#[derive(Debug, Clone)]
pub struct Analysis {
	pub metadata: tg::package::Metadata,
	pub dependencies: Vec<tg::Dependency>,
}

/// Errors that may arise during version solving.
#[derive(Debug, Clone)]
enum Error {
	/// No version could be found that satisfies all constraints.
	PackageVersionConflict,

	/// A package cycle exists.
	PackageCycleExists {
		/// Represents the terminal edge of cycle in the dependency graph.
		dependant: Dependant,
	},

	/// A nested error that arises during backtracking.
	Backtrack {
		/// The package that we backtracked from.
		package: tg::directory::Id,

		/// The version that was tried previously and failed.
		previous_version: String,

		/// A list of dependencies of `previous_version` that caused an error.
		erroneous_dependencies: Vec<(tg::Dependency, Error)>,
	},

	/// A tangram error.
	Other(tangram_error::Error),
}

#[allow(clippy::too_many_lines)]
pub async fn new(
	client: &dyn tg::Client,
	dependency: &tg::Dependency,
) -> Result<(tg::Directory, tg::Lock)> {
	// Get the package with its path dependencies.
	let package_with_path_dependencies = if let Some(path) = dependency.path.as_ref().cloned() {
		// If the dependency is a path dependency, then get the package with its path dependencies from the path.
		let path = tokio::fs::canonicalize(PathBuf::from(path))
			.await
			.wrap_err("Failed to canonicalize the path.")?;
		package_with_path_dependencies_for_path(client, &path).await?
	} else {
		// If the dependency is a registry dependency, then get the package from the registry and make the path dependencies be empty.
		let id = client
			.try_get_package(dependency)
			.await?
			.ok_or(tangram_error::error!(
				r#"Could not find package "{dependency}"."#
			))?;
		let package = tg::Directory::with_id(id.clone());
		PackageWithPathDependencies {
			package,
			path_dependencies: BTreeMap::default(),
		}
	};

	// If this is a path dependency, then attempt to get the lockfile from the path.
	let lockfile = 'a: {
		if let Some(path) = dependency.path.as_ref() {
			// Canonicalize the path.
			let path = PathBuf::from(path.clone())
				.canonicalize()
				.wrap_err("Failed to canonicalize the path.")?;

			// Attempt to read the lockfile.
			let lockfile_path = path.join(LOCKFILE_FILE_NAME);
			let exists = tokio::fs::try_exists(&lockfile_path)
				.await
				.wrap_err("Failed to determine if the lockfile exists.")?;
			if !exists {
				break 'a None;
			}
			let lockfile = tokio::fs::read(&lockfile_path)
				.await
				.wrap_err("Failed to read the lockfile.")?;
			let lockfile: Lockfile = serde_json::from_slice(&lockfile)
				.wrap_err("Failed to deserialize the lockfile.")?;

			// Verify that the lockfile's dependencies match the package with path dependencies.
			let matches =
				lockfile_matches(client, &package_with_path_dependencies, &lockfile).await?;
			if !matches {
				break 'a None;
			}

			Some(lockfile)
		} else {
			None
		}
	};

	// Otherwise, create the lockfile.
	let created = lockfile.is_none();
	let lockfile = if let Some(lockfile) = lockfile {
		lockfile
	} else {
		create_lockfile(client, &package_with_path_dependencies).await?
	};

	// If this is a path dependency and the lockfile was just created, then write the lockfile.
	if let Some(path) = dependency.path.as_ref() {
		if created {
			let package_path = PathBuf::from(path.clone())
				.canonicalize()
				.wrap_err("Failed to canonicalize the path.")?;
			let lockfile_path = package_path.join(LOCKFILE_FILE_NAME);
			let lockfile = serde_json::to_vec_pretty(&lockfile)
				.wrap_err("Failed to serialize the lockfile.")?;
			tokio::fs::write(lockfile_path, lockfile)
				.await
				.wrap_err("Failed to write the lockfile.")?;
		}
	}

	// Get the package.
	let package = package_with_path_dependencies.package.clone();

	// Create the lock.
	let lock = create_lock(&package_with_path_dependencies, &lockfile)?;

	// Return.
	Ok((package, lock))
}

async fn package_with_path_dependencies_for_path(
	client: &dyn tg::Client,
	path: &Path,
) -> tangram_error::Result<PackageWithPathDependencies> {
	let mut visited = BTreeMap::default();
	package_with_path_dependencies_for_path_inner(client, path, &mut visited).await
}

#[async_recursion]
async fn package_with_path_dependencies_for_path_inner(
	client: &dyn tg::Client,
	path: &Path,
	visited: &mut BTreeMap<PathBuf, Option<PackageWithPathDependencies>>,
) -> tangram_error::Result<PackageWithPathDependencies> {
	// Check if the path has already been visited.
	match visited.get(path) {
		Some(Some(package_with_path_dependencies)) => {
			return Ok(package_with_path_dependencies.clone())
		},
		Some(None) => {
			return Err(tangram_error::error!(
				"The package has a circular path dependency."
			))
		},
		None => (),
	}

	// Add the path to the visited set with `None` to detect circular dependencies.
	visited.insert(path.to_owned(), None);

	// Create a builder for the package.
	let mut package = tg::directory::Builder::default();

	// Create a queue of module paths to visit and a visited set.
	let mut queue: VecDeque<tg::Path> =
		VecDeque::from(vec![ROOT_MODULE_FILE_NAME.parse().unwrap()]);
	let mut visited_module_paths: HashSet<tg::Path, fnv::FnvBuildHasher> = HashSet::default();

	// Create the path dependencies.
	let mut path_dependencies = BTreeMap::default();

	// Visit each module.
	while let Some(module_path) = queue.pop_front() {
		// Get the module's absolute path.
		let module_absolute_path = path.join(module_path.to_string());
		let module_absolute_path = tokio::fs::canonicalize(&module_absolute_path)
			.await
			.wrap_err("Failed to canonicalize the module path.")?;

		// Add the module to the package directory.
		let artifact = tg::Artifact::check_in(client, &module_absolute_path).await?;
		package = package.add(client, &module_path, artifact).await?;

		// Get the module's text.
		let permit = client.file_descriptor_semaphore().acquire().await;
		let text = tokio::fs::read_to_string(&module_absolute_path)
			.await
			.wrap_err("Failed to read the module.")?;
		drop(permit);

		// Analyze the module.
		let analysis =
			tangram_lsp::Module::analyze(text).wrap_err("Failed to analyze the module.")?;

		// Handle the includes.
		for include_path in analysis.includes {
			// Get the included artifact's path in the package.
			let included_artifact_path = module_path
				.clone()
				.parent()
				.join(include_path.clone())
				.normalize();

			// Get the included artifact's path.
			let included_artifact_absolute_path = path.join(included_artifact_path.to_string());

			// Check in the artifact at the included path.
			let included_artifact =
				tg::Artifact::check_in(client, &included_artifact_absolute_path).await?;

			// Add the included artifact to the directory.
			package = package
				.add(client, &included_artifact_path, included_artifact)
				.await?;
		}

		// Recurse into the path dependencies.
		for import in &analysis.imports {
			if let tangram_lsp::Import::Dependency(
				dependency @ tg::Dependency { path: Some(_), .. },
			) = import
			{
				// Make the dependency path relative to the package.
				let mut dependency = dependency.clone();
				dependency.path.replace(
					module_path
						.clone()
						.parent()
						.join(dependency.path.as_ref().unwrap().clone())
						.normalize(),
				);

				// Get the dependency's absolute path.
				let dependency_path = path.join(dependency.path.as_ref().unwrap().to_string());
				let dependency_absolute_path = tokio::fs::canonicalize(&dependency_path)
					.await
					.wrap_err("Failed to canonicalize the dependency path.")?;

				// Recurse into the path dependency.
				let child = package_with_path_dependencies_for_path_inner(
					client,
					&dependency_absolute_path,
					visited,
				)
				.await?;

				// Insert the path dependency.
				path_dependencies.insert(dependency, child);
			}
		}

		// Add the module path to the visited set.
		visited_module_paths.insert(module_path.clone());

		// Add the unvisited path imports to the queue.
		for import in &analysis.imports {
			if let tangram_lsp::Import::Module(import) = import {
				let imported_module_path = module_path
					.clone()
					.parent()
					.join(import.clone())
					.normalize();
				if !visited_module_paths.contains(&imported_module_path) {
					queue.push_back(imported_module_path);
				}
			}
		}
	}

	// Create the package.
	let package = package.build();

	// Create the package with path dependencies.
	let package_with_path_dependencies = PackageWithPathDependencies {
		package,
		path_dependencies,
	};

	// Mark the package with path dependencies as visited.
	visited.insert(
		path.to_owned(),
		Some(package_with_path_dependencies.clone()),
	);

	Ok(package_with_path_dependencies)
}

async fn lockfile_matches(
	client: &dyn Client,
	package_with_path_dependencies: &PackageWithPathDependencies,
	lockfile: &Lockfile,
) -> Result<bool> {
	lockfile_matches_inner(client, package_with_path_dependencies, lockfile, 0).await
}

#[async_recursion]
async fn lockfile_matches_inner(
	client: &dyn Client,
	package_with_path_dependencies: &PackageWithPathDependencies,
	lockfile: &Lockfile,
	index: usize,
) -> Result<bool> {
	// Get the package's dependencies.
	let dependencies = package_with_path_dependencies
		.package
		.dependencies(client)
		.await?;

	// Get the package's lock from the lockfile.
	let lock = lockfile.locks.get(index).wrap_err("Invalid lockfile.")?;

	// Verify that the dependencies match.
	if !itertools::equal(lock.dependencies.keys(), dependencies.iter()) {
		return Ok(false);
	}

	// Recurse into the path dependencies.
	package_with_path_dependencies
		.path_dependencies
		.keys()
		.map(|dependency| {
			let dependencies = &dependencies;
			async move {
				let index = lock.dependencies.get(dependency).unwrap().lock;
				lockfile_matches_inner(client, package_with_path_dependencies, lockfile, index)
					.await
			}
		})
		.collect::<FuturesUnordered<_>>()
		.try_all(|matches| async move { matches })
		.await?;

	Ok(true)
}

async fn create_lockfile(
	client: &dyn tg::Client,
	package_with_path_dependencies: &PackageWithPathDependencies,
) -> Result<Lockfile> {
	// Construct the version solving context and working set.
	let mut analysis = BTreeMap::new();
	let mut path_dependencies = BTreeMap::new();
	let mut working_set = im::Vector::new();
	scan_package_with_path_dependencies(client, package_with_path_dependencies, &mut analysis, &mut path_dependencies, &mut working_set).await?;
	let published_packages = HashMap::new();
	let mut context = Context {
		client: client.clone_box(),
		analysis,
		published_packages,
		path_dependencies,
	};

	// Solve.
	let solution = solve_inner(&mut context, working_set).await?;

	// Create the error report.
	let errors = solution
		.partial
		.iter()
		.filter_map(|(dependant, partial)| match partial {
			Mark::Permanent(Err(e)) => Some((dependant.clone(), e.clone())),
			_ => None,
		})
		.collect::<Vec<_>>();

	// If the report is not empty, return an error.
	if !errors.is_empty() {
		let report = Report {
			errors,
			context,
			solution,
		};
		return_error!("{report}");
	}

	// Create the set of locks for all dependencies.
	let mut locks = Vec::new();
	for package in context.path_dependencies.keys().cloned() {
		create_lockfile_inner(client, package, &context, &solution, &mut locks).await?;
	}
	Ok(Lockfile { locks })
}

#[allow(clippy::only_used_in_recursion)]
#[async_recursion]
async fn create_lockfile_inner(
	client: &dyn tg::Client,
	package: tg::directory::Id,
	context: &Context,
	solution: &Solution,
	locks: &mut Vec<Lock>
) -> Result<usize> {
	let analysis = context.analysis.get(&package)
		.wrap_err("Missing package in solution.")?;
	let path_dependencies = context.path_dependencies.get(&package);
	let mut dependencies = BTreeMap::new();
	for dependency in &analysis.dependencies {
		let entry = match (dependency.path.as_ref(), path_dependencies) {
			(Some(path), Some(path_dependencies)) if path_dependencies.contains_key(path) => {
				// Resolve by path.
				let resolved = path_dependencies.get(path).unwrap();
				let lock = create_lockfile_inner(client, resolved.clone(), context, solution, locks).await?;
				Entry {
					package: Some(resolved.clone()),
					lock
				}

			}
			_ => {
				// Resolve by dependant.
				let dependant = Dependant {
					package: package.clone(),
					dependency: dependency.clone()
				};
				let Some(Mark::Permanent(Ok(resolved))) = solution.partial.get(&dependant) else {
					return_error!("Missing solution for {dependant:?}.");
				};
				let lock = create_lockfile_inner(client, resolved.clone(), context, solution, locks).await?;
				Entry {
					package: Some(resolved.clone()),
					lock
				}
			}
		};
		dependencies.insert(dependency.clone(), entry);
	}
	let lock = Lock { dependencies };
	let index = if let Some(index) = locks.iter().position(|l| l == &lock) {
		index
	} else {
		locks.push(lock);
		locks.len() - 1
	};
	Ok(index)
}

#[async_recursion]
async fn scan_package_with_path_dependencies(
	client: &dyn tg::Client,
	package_with_path_dependencies: &PackageWithPathDependencies,
	all_analysis: &mut BTreeMap<tg::directory::Id, Analysis>,
	all_path_dependencies: &mut BTreeMap<tg::directory::Id, BTreeMap<tg::Path, tg::directory::Id>>,
	working_set: &mut im::Vector<Dependant>,
) -> Result<()> {
	let PackageWithPathDependencies {
		package,
		path_dependencies,
	} = package_with_path_dependencies;
	let package_id = package.id(client).await?.clone();

	// Check if we've already visited this dependency.
	if all_path_dependencies.contains_key(&package_id) {
		return Ok(());
	}

	// Get the metadata and dependenencies of this package.
	let dependency = tg::Dependency::with_id(package_id.clone());
	let metadata = client.get_package_metadata(&dependency).await?;
	let dependencies = client.get_package_dependencies(&dependency).await?;

	// Convert dependencies to dependants and update the working set.
	let dependants = dependencies.iter().map(|dependency| Dependant {
		package: package_id.clone(),
		dependency: dependency.clone(),
	});
	working_set.extend(dependants);

	// Add the metadata and dependencies to the analysis cache.
	let analysis = Analysis {
		metadata,
		dependencies,
	};
	all_analysis.insert(package_id.clone(), analysis);

	// Recurse.
	for (dependency, package_with_path_dependencies) in path_dependencies {
		let path = dependency.path.as_ref().unwrap();
		let dependency_package_id = package_with_path_dependencies
			.package
			.id(client)
			.await?
			.clone();
		all_path_dependencies
			.entry(package_id.clone())
			.or_default()
			.insert(path.clone(), dependency_package_id);

		scan_package_with_path_dependencies(client, package_with_path_dependencies, all_analysis, all_path_dependencies, working_set)
			.await?;
	}

	Ok(())
}

// pub(crate) async fn create_lockfile(
// 	_client: &dyn tg::Client,
// 	_package_with_path_dependencies: PackageWithPathDependencies,
// ) -> Result<Lockfile> {
// 	// Create the context.
// 	let mut context = Context::new(client, path_dependencies.clone());

// 	// Seed the context.
// 	let roots = std::iter::once(&root)
// 		.chain(path_dependencies.keys())
// 		.chain(
// 			path_dependencies
// 				.values()
// 				.flat_map(std::collections::BTreeMap::values),
// 		);
// 	for root in roots {
// 		context
// 			.analysis(root)
// 			.await
// 			.wrap_err("Failed to analyze the root package.")?;
// 	}

// 	// Create the initial set of dependants to solve, one for each direct registry dependency of each path dependency.
// 	let working_set = registry_dependencies
// 		.into_iter()
// 		.map(|(package, dependency)| Dependant {
// 			package,
// 			dependency,
// 		})
// 		.collect();

// 	// Solve.
// 	let solution = solve_inner(&mut context, working_set).await?;

// 	// Create the error report.
// 	let errors = solution
// 		.partial
// 		.iter()
// 		.filter_map(|(dependant, partial)| match partial {
// 			Mark::Permanent(Err(e)) => Some((dependant.clone(), e.clone())),
// 			_ => None,
// 		})
// 		.collect::<Vec<_>>();

// 	// If the report is not empty, return an error.
// 	if !errors.is_empty() {
// 		let report = Report {
// 			errors,
// 			context,
// 			solution,
// 		};
// 		tangram_error::return_error!("{report}");
// 	}

// 	// Create the locks.
// 	let mut locks = vec![(".".parse().unwrap(), lock(&context, &solution, root).await?)];
// 	for (_, dependencies) in path_dependencies {
// 		for (path, package) in dependencies {
// 			let lock = lock(&context, &solution, package).await?;
// 			locks.push((path, lock));
// 		}
// 	}

// 	// Create the lockfile.
// 	let paths_ = locks;
// 	let lockfile = {
// 		let mut paths = BTreeMap::new();
// 		let mut locks = BTreeMap::new();
// 		let mut queue = VecDeque::new();
// 		let mut visited = BTreeSet::new();

// 		// Create the paths.
// 		for (path, lock) in paths_ {
// 			let id = lock.id(client).await?.clone();
// 			paths.insert(path, id);
// 			queue.push_back(lock);
// 		}

// 		// Create the locks.
// 		while let Some(next) = queue.pop_front() {
// 			let id = next.id(client).await?;
// 			if visited.contains(id) {
// 				continue;
// 			}
// 			visited.insert(id.clone());
// 			let mut entry_ = BTreeMap::new();
// 			for (dependency, entry) in next.dependencies(client).await? {
// 				queue.push_back(entry.lock.clone());
// 				let entry = tg::lock::data::Entry {
// 					package: entry.package.id(client).await?.clone(),
// 					lock: entry.lock.id(client).await?.clone(),
// 				};
// 				entry_.insert(dependency.clone(), entry.clone());
// 			}
// 			locks.insert(id.clone(), entry_);
// 		}

// 		// Return the lockfile.
// 		Ok::<_, tangram_error::Error>(Lockfile { paths, locks })
// 	}?;

// 	Ok(lockfile)
// }

#[allow(clippy::too_many_lines)]
async fn solve_inner(
	context: &mut Context,
	working_set: im::Vector<Dependant>,
) -> Result<Solution> {
	// Create the first stack frame.
	let solution = Solution::default();
	let last_error = None;
	let remaining_versions = None;
	let mut current_frame = Frame {
		solution,
		working_set,
		remaining_versions,
		last_error,
	};
	let mut history = im::Vector::new();

	// The main driver loop operates on the current stack frame, and iteratively tries to build up the next stack frame.
	while let Some((new_working_set, dependant)) = current_frame.next_dependant() {
		let mut next_frame = Frame {
			working_set: new_working_set,
			solution: current_frame.solution.clone(),
			remaining_versions: None,
			last_error: None,
		};

		let permanent = current_frame.solution.get_permanent(context, &dependant);

		let partial = current_frame.solution.partial.get(&dependant);
		match (permanent, partial) {
			// Case 0: There is no solution for this package yet.
			(None, None) => 'a: {
				// Note: this bit is tricky. The next frame will always have an empty set of remaining versions, because by construction it will never have been tried before. However we need to get a list of versions to attempt, which we will push onto the stack.
				if current_frame.remaining_versions.is_none() {
					let all_versions = match context
						.lookup(dependant.dependency.name.as_ref().unwrap())
						.await
					{
						Ok(all_versions) => all_versions,
						Err(e) => {
							tracing::error!(?dependant, ?e, "Failed to get versions of package.");

							// We cannot solve this dependency.
							current_frame.solution.mark_permanently(
								context,
								dependant,
								Err(Error::Other(e)),
							);

							// This is ugly, but writing out the full match statemenet is uglier and we already have a deeply nested tree of branches.
							break 'a;
						},
					};

					let remaining_versions = all_versions
						.into_iter()
						.filter_map(|version| {
							// TODO: handle the error here. If the published version cannot be parsed then we can continue the loop. If the dependency version cannot be parsed we need to issue a hard error and break out of the match statement.
							let version = version.version.as_deref()?;
							Context::matches(version, &dependant.dependency)
								.ok()?
								.then_some(version.to_owned())
						})
						.collect();
					current_frame.remaining_versions = Some(remaining_versions);
				}

				// Try and pick a version.
				let package = context
					.try_resolve(
						&dependant,
						current_frame.remaining_versions.as_mut().unwrap(),
					)
					.await;

				match package {
					// We successfully popped a version.
					Ok(package) => {
						next_frame.solution = next_frame
							.solution
							.mark_temporarily(dependant.clone(), package.clone());

						// Add this dependency to the top of the stack before adding all its dependencies.
						next_frame.working_set.push_back(dependant.clone());

						// Add all the dependencies to the stack.
						for child_dependency in context.dependencies(&package).await.unwrap() {
							let dependant = Dependant {
								package: package.clone(),
								dependency: child_dependency.clone(),
							};
							next_frame.working_set.push_back(dependant);
						}

						// Update the solution
						next_frame.solution =
							next_frame.solution.mark_temporarily(dependant, package);

						// Update the stack. If we backtrack, we use the next version in the version stack.
						history.push_back(current_frame.clone());
					},

					Err(e) => {
						tracing::error!(?dependant, ?e, "No solution exists.");
						next_frame.solution =
							next_frame
								.solution
								.mark_permanently(context, dependant, Err(e));
					},
				}
			},

			// Case 1: There exists a global version for the package but we haven't solved this dependency constraint.
			(Some(permanent), None) => {
				tracing::debug!(?dependant, ?permanent, "Existing solution found.");
				match permanent {
					// Case 1.1: The happy path. Our version is solved and it matches this constraint.
					Ok(package) => {
						// Successful caches of the version will be memoized, so it's safe to  unwrap here. Annoyingly, borrowck fails here because it doesn't know that the result holds a mutable reference to the context.
						let version = context.version(package).await.unwrap().to_owned();

						// Case 1.1: The happy path. Our version is solved and it matches this constraint.
						match Context::matches(&version, &dependant.dependency) {
							Ok(true) => {
								next_frame.solution = next_frame.solution.mark_permanently(
									context,
									dependant,
									Ok(package.clone()),
								);
							},
							// Case 1.3: The unhappy path. We need to fail.
							Ok(false) => {
								let error = Error::PackageVersionConflict;
								if let Some(frame_) = try_backtrack(
									&history,
									dependant.dependency.name.as_ref().unwrap(),
									error.clone(),
								) {
									next_frame = frame_;
								} else {
									tracing::error!(?dependant, "No solution exists.");
									// There is no solution for this package. Add an error.
									next_frame.solution = next_frame.solution.mark_permanently(
										context,
										dependant,
										Err(error),
									);
								}
							},
							Err(e) => {
								tracing::error!(?dependant, ?e, "Existing solution is an error.");
								next_frame.solution.mark_permanently(
									context,
									dependant,
									Err(Error::Other(e)),
								);
							},
						}
					},
					// Case 1.2: The less happy path. We know there's no solution to this package because we've already failed to satisfy some other set of constraints.
					Err(e) => {
						next_frame.solution = next_frame.solution.mark_permanently(
							context,
							dependant,
							Err(e.clone()),
						);
					},
				}
			},

			// Case 2: We only have a partial solution for this dependency and need to make sure we didn't create a cycle.
			(_, Some(Mark::Temporary(package))) => {
				// Note: it is safe to unwrap here because a successful query to context.dependencies is memoized.
				let dependencies = context.dependencies(package).await.unwrap();

				let mut erroneous_children = vec![];
				for child_dependency in dependencies {
					let child_dependant = Dependant {
						package: package.clone(),
						dependency: child_dependency.clone(),
					};

					let child = next_frame.solution.partial.get(&child_dependant).unwrap();
					match child {
						// The child dependency has been solved.
						Mark::Permanent(Ok(_)) => (),

						// The child dependency has been solved, but it is an error.
						Mark::Permanent(Err(e)) => {
							let error = e.clone();
							erroneous_children.push((child_dependant.dependency, error));
						},

						// The child dependency has not been solved.
						Mark::Temporary(_version) => {
							// Uh oh. We've detected a cycle. First try and backtrack. If backtracking fails, bail out.
							let error = Error::PackageCycleExists {
								dependant: dependant.clone(),
							};
							erroneous_children.push((child_dependant.dependency, error));
						},
					}
				}

				// If none of the children contain errors, we mark this edge permanently
				if erroneous_children.is_empty() {
					next_frame.solution = next_frame.solution.mark_permanently(
						context,
						dependant,
						Ok(package.clone()),
					);
				} else {
					// Successful lookups of the version are memoized, so it's safe to unwrap here.
					let previous_version = context.version(package).await.unwrap().into();
					let error = Error::Backtrack {
						package: package.clone(),
						previous_version,
						erroneous_dependencies: erroneous_children,
					};

					if let Some(frame_) = try_backtrack(
						&history,
						dependant.dependency.name.as_ref().unwrap(),
						error.clone(),
					) {
						next_frame = frame_;
					} else {
						// This means that backtracking totally failed and we need to fail with an error
						next_frame.solution =
							next_frame
								.solution
								.mark_permanently(context, dependant, Err(error));
					}
				}
			},

			// Case 3: We've already solved this dependency. Continue.
			(_, Some(Mark::Permanent(_complete))) => (),
		}

		// Replace the solution and working set if needed.
		current_frame = next_frame;
	}

	Ok(current_frame.solution)
}

impl Context {
	#[async_recursion]
	async fn add_path_dependencies(
		&mut self,
		package_with_path_dependencies: &PackageWithPathDependencies,
	) -> Result<()> {
		let id = package_with_path_dependencies
			.package
			.id(self.client.as_ref())
			.await?
			.clone();
		if self.path_dependencies.contains_key(&id) {
			return Ok(());
		}
		let mut entries = BTreeMap::new();
		for (dependency, next) in &package_with_path_dependencies.path_dependencies {
			let path = dependency.path.as_ref().unwrap();
			self.add_path_dependencies(next).await?;
			let id = next.package.id(self.client.as_ref()).await?.clone();
			entries.insert(path.clone(), id);
		}
		self.path_dependencies.insert(id, entries);
		Ok(())
	}

	#[must_use]
	fn is_path_dependency(&self, dependant: &Dependant) -> bool {
		self.path_dependencies.get(&dependant.package).is_some()
			&& dependant.dependency.path.is_some()
	}

	#[must_use]
	fn resolve_path_dependency(
		&self,
		dependant: &Dependant,
	) -> Option<Result<tg::directory::Id, Error>> {
		let Dependant {
			package,
			dependency,
		} = dependant;
		if let (Some(path_dependencies), Some(path)) = (
			self.path_dependencies.get(package),
			dependency.path.as_ref(),
		) {
			let result = path_dependencies
				.get(path)
				.cloned()
				.ok_or(Error::Other(error!(
					"Could not resolve path dependency for {dependency}."
				)));
			Some(result)
		} else {
			None
		}
	}

	// Check if a package satisfies a dependency.
	fn matches(version: &str, dependency: &tg::Dependency) -> Result<bool> {
		let Some(constraint) = dependency.version.as_ref() else {
			return Ok(true);
		};
		let version: semver::Version = version.parse().map_err(|e| {
			tracing::error!(?e, ?version, "Failed to parse metadata version.");
			error!("Failed to parse version: {version}.")
		})?;
		let constraint: semver::VersionReq = constraint.parse().map_err(|e| {
			tracing::error!(?e, ?dependency, "Failed to parse dependency version.");
			error!("Failed to parse version.")
		})?;

		Ok(constraint.matches(&version))
	}

	// Try and get the next version from a list of remaining ones. Returns an error if the list is empty.
	async fn try_resolve(
		&mut self,
		dependant: &Dependant,
		remaining_versions: &mut im::Vector<String>,
	) -> Result<tg::directory::Id, Error> {
		let name = dependant.dependency.name.as_ref().unwrap();

		// First check if we have a path dependency table for this package and this is a path dependency. If we cannot look up the path dependency we return an error.
		if let Some(result) = self.resolve_path_dependency(dependant) {
			return result;
		}

		// If the cache doesn't contain the package, we need to go out to the client to retrieve the ID. If this errors, we return immediately. If there is no package available for this version (which is extremely unlikely) we loop until we get the next version that's either in the cache, or available from the client.
		loop {
			let version = remaining_versions
				.pop_back()
				.ok_or(Error::PackageVersionConflict)?;
			let metadata = tg::package::Metadata {
				name: Some(name.into()),
				version: Some(version.clone()),
				description: None,
			};
			if let Some(package) = self.published_packages.get(&metadata) {
				return Ok(package.clone());
			}
			let dependency = tg::Dependency {
				name: Some(name.into()),
				version: Some(version.clone()),
				id: None,
				path: None,
			};
			match self.client.get_package(&dependency).await {
				Err(error) => {
					tracing::error!(
						?error,
						?name,
						?version,
						"Failed to get an artifact for the package."
					);
					return Err(Error::Other(error));
				},
				Ok(package) => {
					self.published_packages.insert(metadata, package.clone());
					return Ok(package);
				},
			}
		}
	}

	async fn analysis(&mut self, package: &tg::directory::Id) -> Result<&'_ Analysis> {
		if !self.analysis.contains_key(package) {
			let dependency = Dependency::with_id(package.clone());
			let metadata = self.client.get_package_metadata(&dependency).await?;
			let dependencies = self
				.client
				.get_package_dependencies(&dependency)
				.await?
				.into_iter()
				.filter(|dependency| {
					!(self.path_dependencies.contains_key(package) && dependency.path.is_some())
				})
				.collect();
			let analysis = Analysis {
				metadata: metadata.clone(),
				dependencies,
			};
			self.published_packages
				.insert(metadata.clone(), package.clone());
			self.analysis.insert(package.clone(), analysis);
		}
		Ok(self.analysis.get(package).unwrap())
	}

	// Get a list of registry dependencies for a package given its metadata.
	async fn dependencies(&mut self, package: &tg::directory::Id) -> Result<&'_ [tg::Dependency]> {
		Ok(&self.analysis(package).await?.dependencies)
	}

	async fn version(&mut self, package: &tg::directory::Id) -> Result<&str> {
		self.analysis(package).await?.version()
	}

	// Lookup all the published versions of a package by name.
	async fn lookup(&mut self, package_name: &str) -> Result<Vec<tg::package::Metadata>> {
		let dependency = tg::Dependency {
			name: Some(package_name.into()),
			id: None,
			path: None,
			version: None,
		};
		let metadata = self
			.client
			.get_package_versions(&dependency)
			.await?
			.into_iter()
			.map(|version| tg::package::Metadata {
				name: Some(package_name.into()),
				version: Some(version),
				description: None,
			})
			.collect::<Vec<_>>();
		Ok(metadata)
	}
}

fn try_backtrack(history: &im::Vector<Frame>, package: &str, error: Error) -> Option<Frame> {
	let idx = history
		.iter()
		.take_while(|frame| !frame.solution.contains(package))
		.count();
	let mut frame = history.get(idx).cloned()?;
	frame.last_error = Some(error);
	Some(frame)
}

impl Solution {
	// If there's an existing solution for this dependant, return it. Path dependencies are ignored.
	fn get_permanent(
		&self,
		context: &Context,
		dependant: &Dependant,
	) -> Option<&Result<tg::directory::Id, Error>> {
		if context.is_path_dependency(dependant) {
			return None;
		}
		self.permanent.get(dependant.dependency.name.as_ref()?)
	}

	/// Mark this dependant with a temporary solution.
	fn mark_temporarily(&self, dependant: Dependant, package: tg::directory::Id) -> Self {
		let mut solution = self.clone();
		solution.partial.insert(dependant, Mark::Temporary(package));
		solution
	}

	/// Mark the dependant permanently, adding it to the list of known solutions and the partial solutions.
	fn mark_permanently(
		&self,
		context: &Context,
		dependant: Dependant,
		complete: Result<tg::directory::Id, Error>,
	) -> Self {
		let mut solution = self.clone();

		// Update the global solution.
		if !context.is_path_dependency(&dependant) {
			solution
				.permanent
				.insert(dependant.dependency.name.clone().unwrap(), complete.clone());
		}

		// Update the local solution.
		solution
			.partial
			.insert(dependant, Mark::Permanent(complete));

		solution
	}

	fn contains(&self, package: &str) -> bool {
		self.permanent.contains_key(package)
	}
}

impl Frame {
	fn next_dependant(&self) -> Option<(im::Vector<Dependant>, Dependant)> {
		let mut working_set = self.working_set.clone();
		let dependant = working_set.pop_back()?;
		Some((working_set, dependant))
	}
}

impl std::fmt::Display for Report {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for (dependant, error) in &self.errors {
			self.format(f, dependant, error)?;
		}
		Ok(())
	}
}

impl Report {
	fn format(
		&self,
		f: &mut std::fmt::Formatter<'_>,
		dependant: &Dependant,
		error: &Error,
	) -> std::fmt::Result {
		let Dependant {
			package,
			dependency,
		} = dependant;

		let metadata = &self.context.analysis.get(package).unwrap().metadata;
		let name = metadata.name.as_ref().unwrap();
		let version = metadata.version.as_ref().unwrap();
		write!(f, "{name} @ {version} requires {dependency}, but ")?;

		match error {
			Error::PackageCycleExists { dependant } => {
				let metadata = &self
					.context
					.analysis
					.get(&dependant.package)
					.unwrap()
					.metadata;
				let name = metadata.name.as_ref().unwrap();
				let version = metadata.version.as_ref().unwrap();
				writeln!(f, "{name}@{version}, which creates a cycle.")
			},
			Error::PackageVersionConflict => {
				writeln!(
					f,
					"no version could be found that satisfies the constraints."
				)?;
				let shared_dependants = self
					.solution
					.partial
					.keys()
					.filter(|dependant| dependant.dependency.name == dependency.name);
				for shared in shared_dependants {
					let Dependant {
						package,
						dependency,
						..
					} = shared;
					let metadata = &self.context.analysis.get(package).unwrap().metadata;
					let name = metadata.name.as_ref().unwrap();
					let version = metadata.version.as_ref().unwrap();
					writeln!(f, "{name} @ {version} requires {dependency}")?;
				}
				Ok(())
			},
			Error::Backtrack {
				package,
				previous_version,
				erroneous_dependencies,
			} => {
				writeln!(
					f,
					"{} {previous_version} has errors:",
					dependency.name.as_ref().unwrap()
				)?;
				for (child, error) in erroneous_dependencies {
					let dependant = Dependant {
						package: package.clone(),
						dependency: child.clone(),
					};
					self.format(f, &dependant, error)?;
				}
				Ok(())
			},
			Error::Other(e) => writeln!(f, "{e}"),
		}
	}
}

impl std::fmt::Display for Mark {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Permanent(complete) => write!(f, "Complete({complete:?})"),
			Self::Temporary(version) => write!(f, "Incomplete({version})"),
		}
	}
}

impl Analysis {
	pub fn name(&self) -> Result<&str> {
		self.metadata
			.name
			.as_deref()
			.ok_or(tangram_error::error!("Missing package name."))
	}

	pub fn version(&self) -> Result<&str> {
		self.metadata
			.version
			.as_deref()
			.ok_or(tangram_error::error!("Missing package version."))
	}

	pub fn registry_dependencies(&self) -> impl Iterator<Item = &'_ tg::Dependency> {
		self.dependencies
			.iter()
			.filter(|dependency| dependency.path.is_none())
	}
}

fn create_lock(
	package_with_path_dependencies: &PackageWithPathDependencies,
	lockfile: &Lockfile,
) -> Result<tg::Lock> {
	create_lock_inner(package_with_path_dependencies, lockfile, 0)
}

fn create_lock_inner(
	package_with_path_dependencies: &PackageWithPathDependencies,
	lockfile: &Lockfile,
	index: usize,
) -> Result<tg::Lock> {
	let lock = lockfile.locks.get(index).wrap_err("Invalid lockfile.")?;
	let dependencies = lock
		.dependencies
		.iter()
		.map(|(dependency, entry)| -> Result<_> {
			let (package, lock) = if let Some(package) = entry.package.as_ref() {
				let package = tg::Directory::with_id(package.clone());
				let lock = create_lock_inner(package_with_path_dependencies, lockfile, entry.lock)?;
				(package, lock)
			} else {
				let package_with_path_dependencies = package_with_path_dependencies
					.path_dependencies
					.get(dependency)
					.wrap_err("Missing path dependency.")?;
				let package = package_with_path_dependencies.package.clone();
				let lock = create_lock_inner(package_with_path_dependencies, lockfile, entry.lock)?;
				(package, lock)
			};
			let entry = tg::lock::Entry { package, lock };
			Ok((dependency.clone(), entry))
		})
		.try_collect()?;
	Ok(tg::Lock::with_object(tg::lock::Object { dependencies }))
}

#[async_trait]
impl Ext for tg::Directory {
	async fn dependencies(&self, client: &dyn tg::Client) -> Result<Vec<tg::Dependency>> {
		// Create the dependencies set.
		let mut dependencies: BTreeSet<tg::Dependency> = BTreeSet::default();

		// Create a queue of module paths to visit and a visited set.
		let mut queue: VecDeque<tg::Path> =
			VecDeque::from(vec![ROOT_MODULE_FILE_NAME.parse().unwrap()]);
		let mut visited: HashSet<tg::Path, fnv::FnvBuildHasher> = HashSet::default();

		// Visit each module.
		while let Some(module_path) = queue.pop_front() {
			// Get the file.
			let file = self
				.get(client, &module_path.clone())
				.await?
				.try_unwrap_file()
				.ok()
				.wrap_err("Expected the module to be a file.")?;
			let text = file.text(client).await?;

			// Analyze the module.
			let analysis =
				tangram_lsp::Module::analyze(text).wrap_err("Failed to analyze the module.")?;

			// Recurse into the dependencies.
			for import in &analysis.imports {
				if let tangram_lsp::Import::Dependency(dependency) = import {
					let mut dependency = dependency.clone();

					// Normalize the path dependency to be relative to the root.
					dependency.path = dependency
						.path
						.take()
						.map(|path| module_path.clone().parent().join(path).normalize());

					dependencies.insert(dependency.clone());
				}
			}

			// Add the module path to the visited set.
			visited.insert(module_path.clone());

			// Add the unvisited module imports to the queue.
			for import in &analysis.imports {
				if let tangram_lsp::Import::Module(import) = import {
					let imported_module_path = module_path
						.clone()
						.parent()
						.join(import.clone())
						.normalize();
					if !visited.contains(&imported_module_path) {
						queue.push_back(imported_module_path);
					}
				}
			}
		}

		Ok(dependencies.into_iter().collect())
	}

	async fn metadata(&self, client: &dyn tg::Client) -> Result<tg::package::Metadata> {
		let path = ROOT_MODULE_FILE_NAME.parse().unwrap();
		let file = self
			.get(client, &path)
			.await?
			.try_unwrap_file()
			.ok()
			.wrap_err("Expected the module to be a file.")?;
		let text = file.text(client).await?;
		let analysis = tangram_lsp::Module::analyze(text)?;
		if let Some(metadata) = analysis.metadata {
			Ok(metadata)
		} else {
			return_error!("Missing package metadata.")
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::ROOT_MODULE_FILE_NAME;
	use tangram_client as tg;
	use tangram_error::Result;

	#[tokio::test]
	async fn simple_diamond() {
		let client: Box<dyn tg::Client> = todo!();

		create_package(
			client.as_ref(),
			"simple_diamond_A",
			"1.0.0",
			&[
				"simple_diamond_B@^1.0".parse().unwrap(),
				"simple_diamond_C@^1.0".parse().unwrap(),
			],
		)
		.await
		.unwrap();

		create_package(
			client.as_ref(),
			"simple_diamond_B",
			"1.0.0",
			&["simple_diamond_D@^1.0".parse().unwrap()],
		)
		.await
		.unwrap();

		create_package(
			client.as_ref(),
			"simple_diamond_C",
			"1.0.0",
			&["simple_diamond_D@^1.0".parse().unwrap()],
		)
		.await
		.unwrap();

		create_package(client.as_ref(), "simple_diamond_D", "1.0.0", &[])
			.await
			.unwrap();
	}

	#[tokio::test]
	async fn simple_backtrack() {
		let client: Box<dyn tg::Client> = todo!();

		create_package(
			client.as_ref(),
			"simple_backtrack_A",
			"1.0.0",
			&[
				"simple_backtrack_B@^1.2.3".parse().unwrap(),
				"simple_backtrack_C@<1.2.3".parse().unwrap(),
			],
		)
		.await
		.unwrap();

		create_package(
			client.as_ref(),
			"simple_backtrack_B",
			"1.2.3",
			&["simple_backtrack_C@<1.2.3".parse().unwrap()],
		)
		.await
		.unwrap();

		create_package(client.as_ref(), "simple_backtrack_C", "1.2.3", &[])
			.await
			.unwrap();

		create_package(client.as_ref(), "simple_backtrack_C", "1.2.2", &[])
			.await
			.unwrap();
	}

	#[tokio::test]
	async fn diamond_backtrack() {
		let client: Box<dyn tg::Client> = todo!();

		create_package(
			client.as_ref(),
			"diamond_backtrack_A",
			"1.0.0",
			&[
				"diamond_backtrack_B@1.0.0".parse().unwrap(),
				"diamond_backtrack_C@1.0.0".parse().unwrap(),
			],
		)
		.await
		.unwrap();

		create_package(
			client.as_ref(),
			"diamond_backtrack_B",
			"1.0.0",
			&["diamond_backtrack_D@<1.5.0".parse().unwrap()],
		)
		.await
		.unwrap();

		create_package(
			client.as_ref(),
			"diamond_backtrack_C",
			"1.0.0",
			&["diamond_backtrack_D@<1.3.0".parse().unwrap()],
		)
		.await
		.unwrap();

		create_package(client.as_ref(), "diamond_backtrack_D", "1.1.0", &[])
			.await
			.unwrap();

		create_package(client.as_ref(), "diamond_backtrack_D", "1.2.0", &[])
			.await
			.unwrap();

		create_package(client.as_ref(), "diamond_backtrack_D", "1.3.0", &[])
			.await
			.unwrap();

		create_package(client.as_ref(), "diamond_backtrack_D", "1.4.0", &[])
			.await
			.unwrap();

		create_package(client.as_ref(), "diamond_backtrack_D", "1.5.0", &[])
			.await
			.unwrap();
	}

	#[tokio::test]
	async fn cycle_exists() {
		let client: Box<dyn tg::Client> = todo!();

		create_package(
			client.as_ref(),
			"cycle_exists_A",
			"1.0.0",
			&["cycle_exists_B@1.0.0".parse().unwrap()],
		)
		.await
		.unwrap();

		create_package(
			client.as_ref(),
			"cycle_exists_B",
			"1.0.0",
			&["cycle_exists_C@1.0.0".parse().unwrap()],
		)
		.await
		.unwrap();

		create_package(
			client.as_ref(),
			"cycle_exists_C",
			"1.0.0",
			&["cycle_exists_B@1.0.0".parse().unwrap()],
		)
		.await
		.unwrap();
	}

	#[tokio::test]
	async fn diamond_incompatible_versions() {
		let client: Box<dyn tg::Client> = todo!();
		create_package(
			client.as_ref(),
			"diamond_incompatible_versions_A",
			"1.0.0",
			&[
				"diamond_incompatible_versions_B@1.0.0".parse().unwrap(),
				"diamond_incompatible_versions_C@1.0.0".parse().unwrap(),
			],
		)
		.await
		.unwrap();

		create_package(
			client.as_ref(),
			"diamond_incompatible_versions_B",
			"1.0.0",
			&["diamond_incompatible_versions_D@<1.2.0".parse().unwrap()],
		)
		.await
		.unwrap();

		create_package(
			client.as_ref(),
			"diamond_incompatible_versions_C",
			"1.0.0",
			&["diamond_incompatible_versions_D@>1.3.0".parse().unwrap()],
		)
		.await
		.unwrap();

		create_package(
			client.as_ref(),
			"diamond_incompatible_versions_D",
			"1.0.0",
			&[],
		)
		.await
		.unwrap();

		create_package(
			client.as_ref(),
			"diamond_incompatible_versions_D",
			"1.1.0",
			&[],
		)
		.await
		.unwrap();

		create_package(
			client.as_ref(),
			"diamond_incompatible_versions_D",
			"1.2.0",
			&[],
		)
		.await
		.unwrap();

		create_package(
			client.as_ref(),
			"diamond_incompatible_versions_D",
			"1.3.0",
			&[],
		)
		.await
		.unwrap();

		create_package(
			client.as_ref(),
			"diamond_incompatible_versions_D",
			"1.4.0",
			&[],
		)
		.await
		.unwrap();
	}

	#[tokio::test]
	#[allow(clippy::similar_names)]
	async fn diamond_with_path_dependencies() {
		let client: Box<dyn tg::Client> = todo!();

		let foo = r#"
			export let metadata = {
				name: "foo",
				version: "1.0.0",
			};

			import bar from "tangram:?path=./path/to/bar";
			import baz from "tangram:baz@^1";
			export default tg.target(() => tg`foo ${bar} {baz}`);
		"#;

		let bar = r#"
			export let metadata = {
				name: "bar",
				version: "1.0.0",
			};

			import * as baz from "tangram:baz@=1.2.3";
			export default tg.target(() => tg`bar ${baz}`);
		"#;

		let baz = r#"
			export let metadata = {
				name: "baz",
				version: "1.2.3",
			};

			export default tg.target(() => "baz");
		"#;
	}

	#[tokio::test]
	async fn complex_diamond() {
		let client: Box<dyn tg::Client> = todo!();

		create_package(
			client.as_ref(),
			"complex_diamond_A",
			"1.0.0",
			&[
				"complex_diamond_B@^1.0.0".parse().unwrap(),
				"complex_diamond_E@^1.1.0".parse().unwrap(),
				"complex_diamond_C@^1.0.0".parse().unwrap(),
				"complex_diamond_D@^1.0.0".parse().unwrap(),
			],
		)
		.await
		.unwrap();

		create_package(
			client.as_ref(),
			"complex_diamond_B",
			"1.0.0",
			&["complex_diamond_D@^1.0.0".parse().unwrap()],
		)
		.await
		.unwrap();

		create_package(
			client.as_ref(),
			"complex_diamond_C",
			"1.0.0",
			&[
				"complex_diamond_D@^1.0.0".parse().unwrap(),
				"complex_diamond_E@>1.0.0".parse().unwrap(),
			],
		)
		.await
		.unwrap();

		create_package(
			client.as_ref(),
			"complex_diamond_D",
			"1.3.0",
			&["complex_diamond_E@=1.0.0".parse().unwrap()],
		)
		.await
		.unwrap();

		create_package(
			client.as_ref(),
			"complex_diamond_D",
			"1.2.0",
			&["complex_diamond_E@^1.0.0".parse().unwrap()],
		)
		.await
		.unwrap();

		create_package(client.as_ref(), "complex_diamond_E", "1.0.0", &[])
			.await
			.unwrap();

		create_package(client.as_ref(), "complex_diamond_E", "1.1.0", &[])
			.await
			.unwrap();
	}

	async fn create_package(
		client: &dyn tg::Client,
		name: &str,
		version: &str,
		dependencies: &[tg::Dependency],
	) -> Result<()> {
		let imports = dependencies
			.iter()
			.map(|dep| {
				let name = dep.name.as_ref().unwrap();
				let version = dep.version.as_ref().unwrap();
				format!(r#"import * as {name} from "tangram:{name}@{version}";"#)
			})
			.collect::<Vec<_>>()
			.join("\n");
		let contents = format!(
			r#"
				{imports}
				export let metadata = {{
						name: "{name}",
						version: "{version}",
				}};
				export default tg.target(() => `Hello, from "{name}"!`);
			"#
		);
		let contents = tg::blob::Blob::with_reader(client, contents.as_bytes())
			.await
			.unwrap();
		let file = tg::File::builder(contents).build();
		let package = tg::Directory::with_object(tg::directory::Object {
			entries: [(ROOT_MODULE_FILE_NAME.to_owned(), file.into())]
				.into_iter()
				.collect(),
		});
		let id = package.id(client).await?;
		client.publish_package(None, &id.clone()).await?;
		Ok(())
	}
}
