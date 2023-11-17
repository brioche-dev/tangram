use async_recursion::async_recursion;
use core::fmt;
use im::HashMap;
use std::collections::{BTreeMap, BTreeSet};
use tangram_client as tg;
use tangram_error::{error, return_error, WrapErr};
use tg::Client;

use super::Analysis;

/// Errors that may arise during version solving.
#[derive(Debug, Clone)]
pub enum Error {
	/// The package does not exist in the registry.
	PackageDoesNotExist,

	/// No version could be found that satisfies all constraints.
	PackageVersionConflict,

	/// A package cycle exists.
	PackageCycleExists {
		/// Represents the terminal edge of cycle in the dependency graph.
		dependant: Dependant,
	},

	/// A nested error that arises during backtracking.
	BacktrackError {
		/// The package that we backtracked from.
		package: tg::Id,

		/// The version that was tried previously and failed.
		previous_version: String,

		/// A list of dependencies of `previous_version` that caused an error.
		erroneous_dependencies: Vec<(tg::Dependency, Error)>,
	},

	/// Semantic version parsing error.
	Semver(String),

	/// A tangram error.
	Other(tangram_error::Error),
}

/// Given a registry and unlocked package, create a lockfile for it. If no solution can be found, a [`Report`] containing a description of the most recent set of errors is formatted as an error. On success, a vec of [`tg::Path`]s (relative to the root package), package [`tg::Id`]s, and their [`tg::Lock`]s are returned. There is one vec entry for the root, and one entry for each path dependency.
pub async fn solve(
	client: &dyn Client,
	root: tg::Id,
	path_dependencies: BTreeMap<tg::Id, BTreeMap<tg::Path, tg::Id>>,
	registry_dependencies: BTreeSet<(tg::Id, tg::Dependency)>,
) -> tangram_error::Result<Vec<(tg::Path, tg::Lock)>> {
	// Create the context.
	let mut context = Context::new(client, path_dependencies.clone());

	// Seed the context.
	let roots = std::iter::once(&root)
		.chain(path_dependencies.keys())
		.chain(path_dependencies.values().flat_map(|v| v.values()));
	for root in roots {
		context
			.analysis(root)
			.await
			.wrap_err("Failed to analyze root package.")?;
	}

	// Create the initial set of dependants to solve, one for each direct registry dependency of each path dependency.
	let working_set = registry_dependencies
		.into_iter()
		.map(|(package, dependency)| Dependant {
			package,
			dependency,
		})
		.collect();

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
		tangram_error::return_error!("{report}");
	}

	// Now we have the solution, create a lock.
	let mut locks = vec![(".".parse().unwrap(), lock(&context, &solution, root).await?)];

	for (_, dependencies) in path_dependencies {
		for (relpath, package) in dependencies {
			let lock = lock(&context, &solution, package).await?;
			locks.push((relpath, lock));
		}
	}

	Ok(locks)
}

#[async_recursion]
async fn lock(
	context: &Context,
	solution: &Solution,
	package: tg::Id,
) -> tangram_error::Result<tg::Lock> {
	// Retrieve the dependencies for the package. The unwrap() is safe since we cannot reach this point if a package has not been added to the context.
	let dependencies = context
		.client
		.get_package_dependencies(&package)
		.await?
		.unwrap_or_default();

	// Lock each dependency.
	let mut locked_dependencies = BTreeMap::default();
	for dependency in dependencies {
		let dependant = Dependant {
			package: package.clone(),
			dependency: dependency.clone(),
		};

		// The solution only contains registry dependencies, so we have to make sure to look up the path dependencies in the set of known packages.
		let package = if context.is_path_dependency(&dependant) {
			let dependencies = context.path_dependencies.get(&dependant.package).unwrap();
			dependencies
				.get(dependant.dependency.path.as_ref().unwrap())
				.unwrap()
				.clone()
		} else {
			// The only way for a temporary mark to remain is if the solving algorithm is implemented incorrectly.
			let Some(Mark::Permanent(Ok(package))) = solution.partial.get(&dependant).cloned()
			else {
				return_error!("Internal error, solution is incomplete. package: {dependant:?}");
			};
			package
		};

		let lock = lock(context, solution, package.clone()).await?;
		let id = tg::artifact::Id::try_from(package)
			.wrap_err("Failed to get package artifact from its id.")?;
		let package = tg::Artifact::with_id(id.clone());
		let entry = tg::lock::Entry { package, lock };
		locked_dependencies.insert(dependency.clone(), entry);
	}

	// Create the lock and return.
	let lock = tg::Lock::with_object(tg::lock::Object {
		dependencies: locked_dependencies,
	});
	Ok(lock)
}

#[allow(clippy::too_many_lines)]
async fn solve_inner(
	context: &mut Context,
	working_set: im::Vector<Dependant>,
) -> tangram_error::Result<Solution> {
	// Create the first stack frame.
	let solution = Solution::empty();
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
							context
								.matches(version, &dependant.dependency)
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
						match context.matches(&version, &dependant.dependency) {
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
					let error = Error::BacktrackError {
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

pub struct Context {
	// The backing client.
	client: Box<dyn tg::Client>,

	// A cache of package analysis (metadata, direct dependencies).
	analysis: HashMap<tg::Id, Analysis>,

	// A cache of published packages that we know about.
	published_packages: HashMap<tg::package::Metadata, tg::Id>,

	// The roots of the solve.
	roots: Vec<tg::Id>,

	// A table of path dependencies.
	path_dependencies: BTreeMap<tg::Id, BTreeMap<tg::Path, tg::Id>>,
}

impl fmt::Debug for Context {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Context()")
	}
}

/// The Report is an error type that can be pretty printed to describe why version solving failed.
#[derive(Debug)]
pub struct Report {
	errors: Vec<(Dependant, Error)>,
	context: Context,
	solution: Solution,
}

#[derive(Clone, Debug)]
struct Solution {
	permanent: im::HashMap<String, Result<tg::Id, Error>>,
	partial: im::HashMap<Dependant, Mark>,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Dependant {
	package: tg::Id,
	dependency: tg::Dependency,
}

#[derive(Clone, Debug)]
enum Mark {
	Temporary(tg::Id),
	Permanent(Result<tg::Id, Error>),
}

#[derive(Clone, Debug)]
struct Frame {
	solution: Solution,
	working_set: im::Vector<Dependant>,
	remaining_versions: Option<im::Vector<String>>,
	last_error: Option<Error>,
}

impl Context {
	pub fn new(
		client: &dyn tg::Client,
		path_dependencies: BTreeMap<tg::Id, BTreeMap<tg::Path, tg::Id>>,
	) -> Self {
		let client = client.clone_box();
		let packages = HashMap::new();
		let analysis = HashMap::new();
		let roots = Vec::new();
		Self {
			client,
			published_packages: packages,
			analysis,
			roots,
			path_dependencies,
		}
	}

	#[must_use]
	pub fn is_path_dependency(&self, dependant: &Dependant) -> bool {
		self.path_dependencies.get(&dependant.package).is_some()
			&& dependant.dependency.path.is_some()
	}

	#[must_use]
	pub fn resolve_path_dependency(&self, dependant: &Dependant) -> Option<Result<tg::Id, Error>> {
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

	pub fn add_root(&mut self, package: tg::Id) {
		self.roots.push(package);
	}

	// Check if a package satisfies a dependency.
	#[allow(clippy::unused_self)]
	fn matches(&self, version: &str, dependency: &tg::Dependency) -> tangram_error::Result<bool> {
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
	) -> Result<tg::Id, Error> {
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
			match self.client.get_package_version(name, &version).await {
				Err(e) => {
					tracing::error!(
						?e,
						?name,
						?version,
						"Failed to get an artifact for the package."
					);
					return Err(Error::Other(e));
				},
				Ok(Some(package)) => {
					let package: tg::Id = package.into();
					self.published_packages.insert(metadata, package.clone());
					return Ok(package);
				},
				Ok(None) => continue,
			}
		}
	}

	pub async fn analysis(&mut self, package: &tg::Id) -> tangram_error::Result<&'_ Analysis> {
		if !self.analysis.contains_key(package) {
			let metadata = self
				.client
				.get_package_metadata(package)
				.await?
				.ok_or(error!("Missing package metadata."))?;
			let dependencies = self
				.client
				.get_package_dependencies(package)
				.await?
				.into_iter()
				.flatten()
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
	pub async fn dependencies(
		&mut self,
		package: &tg::Id,
	) -> tangram_error::Result<&'_ [tg::Dependency]> {
		Ok(&self.analysis(package).await?.dependencies)
	}

	pub async fn version(&mut self, package: &tg::Id) -> tangram_error::Result<&str> {
		self.analysis(package).await?.version()
	}

	// Lookup all the published versions of a package by name.
	async fn lookup(
		&mut self,
		package_name: &str,
	) -> tangram_error::Result<Vec<tg::package::Metadata>> {
		let metadata = self
			.client
			.get_package(package_name)
			.await?
			.ok_or(error!("Could not find package named {package_name}."))?
			.versions
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
	fn empty() -> Self {
		Self {
			permanent: im::HashMap::new(),
			partial: im::HashMap::new(),
		}
	}

	// If there's an existing solution for this dependant, return it. Path dependencies are ignored.
	fn get_permanent(
		&self,
		context: &Context,
		dependant: &Dependant,
	) -> Option<&Result<tg::Id, Error>> {
		if context.is_path_dependency(dependant) {
			return None;
		}
		self.permanent.get(dependant.dependency.name.as_ref()?)
	}

	/// Mark this dependant with a temporary solution.
	fn mark_temporarily(&self, dependant: Dependant, package: tg::Id) -> Self {
		let mut solution = self.clone();
		solution.partial.insert(dependant, Mark::Temporary(package));
		solution
	}

	/// Mark the dependant permanently, adding it to the list of known solutions and the partial solutions.
	fn mark_permanently(
		&self,
		context: &Context,
		dependant: Dependant,
		complete: Result<tg::Id, Error>,
	) -> Self {
		let mut solution = self.clone();

		// Update the global solution.
		if !context.is_path_dependency(&dependant) {
			let _old = solution
				.permanent
				.insert(dependant.dependency.name.clone().unwrap(), complete.clone());
		}

		// Update the local solution.
		let _old = solution
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

impl fmt::Display for Report {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		for (dependant, error) in &self.errors {
			self.format(f, dependant, error)?;
		}
		Ok(())
	}
}

impl Report {
	fn format(
		&self,
		f: &mut fmt::Formatter<'_>,
		dependant: &Dependant,
		error: &Error,
	) -> fmt::Result {
		let Dependant {
			package,
			dependency,
		} = dependant;

		let metadata = &self.context.analysis.get(package).unwrap().metadata;
		let name = metadata.name.as_ref().unwrap();
		let version = metadata.version.as_ref().unwrap();
		write!(f, "{name} @ {version} requires {dependency}, but ")?;

		match error {
			Error::Semver(semver) => writeln!(f, "there is a semver error: {semver}."),
			Error::PackageDoesNotExist => {
				writeln!(f, "no package by that name exists in the registry.\n")
			},
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
			Error::BacktrackError {
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

impl fmt::Display for Mark {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Permanent(complete) => write!(f, "Complete({complete:?})"),
			Self::Temporary(version) => write!(f, "Incomplete({version})"),
		}
	}
}
