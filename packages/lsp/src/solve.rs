use crate::package::Analysis;
use async_recursion::async_recursion;
use core::fmt;
use im::HashMap;
use std::collections::{BTreeMap, BTreeSet};
use tangram_client as tg;
use tangram_error::{error, return_error, WrapErr};
use tg::Client;

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

#[cfg(test)]
mod tests {
	use super::solve;
	use crate::ROOT_MODULE_FILE_NAME;
	use async_trait::async_trait;
	use futures::stream::BoxStream;
	use tangram_client as tg;
	use tangram_error::WrapErr;
	use tg::Client;

	#[tokio::test]
	async fn simple_diamond() {
		let client = MockClient::new().await;
		client
			.create_mock_package(
				"simple_diamond_A",
				"1.0.0",
				&[
					tg::Dependency::with_name_and_version(
						"simple_diamond_B".into(),
						Some("^1.0".into()),
					),
					tg::Dependency::with_name_and_version(
						"simple_diamond_C".into(),
						Some("^1.0".into()),
					),
				],
			)
			.await;
		client
			.create_mock_package(
				"simple_diamond_B",
				"1.0.0",
				&[tg::Dependency::with_name_and_version(
					"simple_diamond_D".into(),
					Some("^1.0".into()),
				)],
			)
			.await;

		client
			.create_mock_package(
				"simple_diamond_C",
				"1.0.0",
				&[tg::Dependency::with_name_and_version(
					"simple_diamond_D".into(),
					Some("^1.0".into()),
				)],
			)
			.await;
		client
			.create_mock_package("simple_diamond_D", "1.0.0", &[])
			.await;

		let metadata = tg::package::Metadata {
			name: Some("simple_diamond_A".into()),
			version: Some("1.0.0".into()),
			description: None,
		};

		let _lock = client
			.try_solve(metadata.clone())
			.await
			.expect("Failed to solve simple_diamond case.");
	}

	#[tokio::test]
	async fn simple_backtrack() {
		let client = MockClient::new().await;
		client
			.create_mock_package(
				"simple_backtrack_A",
				"1.0.0",
				&[
					tg::Dependency::with_name_and_version(
						"simple_backtrack_B".into(),
						Some("^1.2.3".into()),
					),
					tg::Dependency::with_name_and_version(
						"simple_backtrack_C".into(),
						Some("<1.2.3".into()),
					),
				],
			)
			.await;
		client
			.create_mock_package(
				"simple_backtrack_B",
				"1.2.3",
				&[tg::Dependency::with_name_and_version(
					"simple_backtrack_C".into(),
					Some("<1.2.3".into()),
				)],
			)
			.await;
		client
			.create_mock_package("simple_backtrack_C", "1.2.3", &[])
			.await;
		client
			.create_mock_package("simple_backtrack_C", "1.2.2", &[])
			.await;

		let metadata = tg::package::Metadata {
			name: Some("simple_backtrack_A".into()),
			version: Some("1.0.0".into()),
			description: None,
		};

		let _lock = client
			.try_solve(metadata.clone())
			.await
			.expect("Failed to solve simple_backtrack case.");
	}

	#[tokio::test]
	async fn diamond_backtrack() {
		let client = MockClient::new().await;
		client
			.create_mock_package(
				"diamond_backtrack_A",
				"1.0.0",
				&[
					tg::Dependency::with_name_and_version(
						"diamond_backtrack_B".into(),
						Some("1.0.0".into()),
					),
					tg::Dependency::with_name_and_version(
						"diamond_backtrack_C".into(),
						Some("1.0.0".into()),
					),
				],
			)
			.await;
		client
			.create_mock_package(
				"diamond_backtrack_B",
				"1.0.0",
				&[tg::Dependency::with_name_and_version(
					"diamond_backtrack_D".into(),
					Some("<1.5.0".into()),
				)],
			)
			.await;
		client
			.create_mock_package(
				"diamond_backtrack_C",
				"1.0.0",
				&[tg::Dependency::with_name_and_version(
					"diamond_backtrack_D".into(),
					Some("<1.3.0".into()),
				)],
			)
			.await;
		client
			.create_mock_package("diamond_backtrack_D", "1.1.0", &[])
			.await;
		client
			.create_mock_package("diamond_backtrack_D", "1.2.0", &[])
			.await;
		client
			.create_mock_package("diamond_backtrack_D", "1.3.0", &[])
			.await;
		client
			.create_mock_package("diamond_backtrack_D", "1.4.0", &[])
			.await;
		client
			.create_mock_package("diamond_backtrack_D", "1.5.0", &[])
			.await;

		let metadata: tg::package::Metadata = tg::package::Metadata {
			name: Some("diamond_backtrack_A".into()),
			version: Some("1.0.0".into()),
			description: None,
		};

		let _lock = client
			.try_solve(metadata.clone())
			.await
			.expect("Failed to solve diamond_backtrack case.");
	}

	#[tokio::test]
	async fn cycle_exists() {
		let client = MockClient::new().await;
		client
			.create_mock_package(
				"cycle_exists_A",
				"1.0.0",
				&[tg::Dependency::with_name_and_version(
					"cycle_exists_B".into(),
					Some("1.0.0".into()),
				)],
			)
			.await;
		client
			.create_mock_package(
				"cycle_exists_B",
				"1.0.0",
				&[tg::Dependency::with_name_and_version(
					"cycle_exists_C".into(),
					Some("1.0.0".into()),
				)],
			)
			.await;
		client
			.create_mock_package(
				"cycle_exists_C",
				"1.0.0",
				&[tg::Dependency::with_name_and_version(
					"cycle_exists_B".into(),
					Some("1.0.0".into()),
				)],
			)
			.await;

		let metadata: tg::package::Metadata = tg::package::Metadata {
			name: Some("cycle_exists_A".into()),
			version: Some("1.0.0".into()),
			description: None,
		};

		let report = client
			.try_solve(metadata.clone())
			.await
			.expect_err("Expected to fail with cycle detection.");

		println!("{report}");
	}

	#[tokio::test]
	async fn diamond_incompatible_versions() {
		let client = MockClient::new().await;
		client
			.create_mock_package(
				"diamond_incompatible_versions_A",
				"1.0.0",
				&[
					tg::Dependency::with_name_and_version(
						"diamond_incompatible_versions_B".into(),
						Some("1.0.0".into()),
					),
					tg::Dependency::with_name_and_version(
						"diamond_incompatible_versions_C".into(),
						Some("1.0.0".into()),
					),
				],
			)
			.await;
		client
			.create_mock_package(
				"diamond_incompatible_versions_B",
				"1.0.0",
				&[tg::Dependency::with_name_and_version(
					"diamond_incompatible_versions_D".into(),
					Some("<1.2.0".into()),
				)],
			)
			.await;
		client
			.create_mock_package(
				"diamond_incompatible_versions_C",
				"1.0.0",
				&[tg::Dependency::with_name_and_version(
					"diamond_incompatible_versions_D".into(),
					Some(">1.3.0".into()),
				)],
			)
			.await;

		client
			.create_mock_package("diamond_incompatible_versions_D", "1.0.0", &[])
			.await;
		client
			.create_mock_package("diamond_incompatible_versions_D", "1.1.0", &[])
			.await;
		client
			.create_mock_package("diamond_incompatible_versions_D", "1.2.0", &[])
			.await;
		client
			.create_mock_package("diamond_incompatible_versions_D", "1.3.0", &[])
			.await;
		client
			.create_mock_package("diamond_incompatible_versions_D", "1.4.0", &[])
			.await;

		let metadata = tg::package::Metadata {
			name: Some("diamond_incompatible_versions_A".into()),
			version: Some("1.0.0".into()),
			description: None,
		};
		let report = client
			.try_solve(metadata.clone())
			.await
			.expect_err("Expected to fail with cycle detection.");

		println!("{report}");
	}

	#[tokio::test]
	#[allow(clippy::similar_names)]
	async fn diamond_with_path_dependencies() {
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
		let client = MockClient::new().await;

		let foo = tg::Directory::new(
			[(
				ROOT_MODULE_FILE_NAME.into(),
				tg::Artifact::from(
					tg::File::builder(
						tg::blob::Blob::with_reader(&client, foo.as_bytes())
							.await
							.unwrap(),
					)
					.build(),
				),
			)]
			.into_iter()
			.collect(),
		);

		let bar = tg::Directory::new(
			[(
				ROOT_MODULE_FILE_NAME.into(),
				tg::Artifact::from(
					tg::File::builder(
						tg::blob::Blob::with_reader(&client, bar.as_bytes())
							.await
							.unwrap(),
					)
					.build(),
				),
			)]
			.into_iter()
			.collect(),
		);

		let baz = tg::Directory::new(
			[(
				ROOT_MODULE_FILE_NAME.into(),
				tg::Artifact::from(
					tg::File::builder(
						tg::blob::Blob::with_reader(&client, baz.as_bytes())
							.await
							.unwrap(),
					)
					.build(),
				),
			)]
			.into_iter()
			.collect(),
		);
		client
			.publish_package("", &baz.id(&client).await.unwrap().clone().into())
			.await
			.unwrap();

		// Create the path dependency table.
		let foo_id: tg::Id = foo.id(&client).await.unwrap().clone().into();
		let foo_path_dependencies: BTreeMap<tg::Path, tg::Id> = [(
			"./path/to/bar".parse().unwrap(),
			bar.id(&client).await.unwrap().clone().into(),
		)]
		.into_iter()
		.collect();
		let path_dependencies = [(foo_id.clone(), foo_path_dependencies)]
			.into_iter()
			.collect();

		// Create the registry dependencies
		let bar_id = bar.id(&client).await.unwrap().clone().into();
		let baz_id = baz.id(&client).await.unwrap().clone().into();
		let registry_dependencies = client
			.registry_dependencies(&[foo_id.clone(), bar_id, baz_id])
			.await;

		// Lock using foo as the root.
		let _lock = solve(&client, foo_id, path_dependencies, registry_dependencies)
			.await
			.expect("Failed to lock diamond with a path dependency.");
	}

	#[tokio::test]
	async fn complex_diamond() {
		let client = MockClient::new().await;
		client
			.create_mock_package(
				"complex_diamond_A",
				"1.0.0",
				&[
					tg::Dependency::with_name_and_version(
						"complex_diamond_B".into(),
						Some("^1.0.0".into()),
					),
					tg::Dependency::with_name_and_version(
						"complex_diamond_E".into(),
						Some("^1.1.0".into()),
					),
					tg::Dependency::with_name_and_version(
						"complex_diamond_C".into(),
						Some("^1.0.0".into()),
					),
					tg::Dependency::with_name_and_version(
						"complex_diamond_D".into(),
						Some("^1.0.0".into()),
					),
				],
			)
			.await;
		client
			.create_mock_package(
				"complex_diamond_B",
				"1.0.0",
				&[tg::Dependency::with_name_and_version(
					"complex_diamond_D".into(),
					Some("^1.0.0".into()),
				)],
			)
			.await;
		client
			.create_mock_package(
				"complex_diamond_C",
				"1.0.0",
				&[
					tg::Dependency::with_name_and_version(
						"complex_diamond_D".into(),
						Some("^1.0.0".into()),
					),
					tg::Dependency::with_name_and_version(
						"complex_diamond_E".into(),
						Some(">1.0.0".into()),
					),
				],
			)
			.await;

		client
			.create_mock_package(
				"complex_diamond_D",
				"1.3.0",
				&[tg::Dependency::with_name_and_version(
					"complex_diamond_E".into(),
					Some("=1.0.0".into()),
				)],
			)
			.await;
		client
			.create_mock_package(
				"complex_diamond_D",
				"1.2.0",
				&[tg::Dependency::with_name_and_version(
					"complex_diamond_E".into(),
					Some("^1.0.0".into()),
				)],
			)
			.await;
		client
			.create_mock_package("complex_diamond_E", "1.0.0", &[])
			.await;
		client
			.create_mock_package("complex_diamond_E", "1.1.0", &[])
			.await;
		let metadata = tg::package::Metadata {
			name: Some("complex_diamond_A".into()),
			version: Some("1.0.0".into()),
			description: None,
		};
		let _lock = client
			.try_solve(metadata.clone())
			.await
			.expect("Failed to solve diamond_backtrack case.");
	}

	use std::{
		collections::{BTreeMap, BTreeSet, HashMap},
		path::{Path, PathBuf},
		sync::{Arc, Mutex},
	};

	/// A test client for debugging the version solving algorithm without requiring a full backend.
	#[derive(Clone)]
	pub struct MockClient {
		state: Arc<Mutex<State>>,
		client: Arc<dyn tg::Client>,
	}

	#[derive(Debug)]
	struct State {
		packages: BTreeMap<String, Vec<MockPackage>>,
		dependencies: HashMap<tg::package::Metadata, Vec<tg::Dependency>>,
	}

	#[derive(Debug)]
	struct MockPackage {
		metadata: tg::package::Metadata,
		artifact: tg::Artifact,
	}

	impl MockClient {
		pub async fn new() -> Self {
			#[allow(clippy::map_unwrap_or)]
			let path = std::env::var("TANGRAM_PATH")
				.map(PathBuf::from)
				.unwrap_or_else(|_| {
					let home = PathBuf::from(std::env::var("HOME").unwrap());
					home.join(".tangram")
				});

			// Attempt to connect to the server.
			let addr = tangram_http::net::Addr::Unix(path.join("socket"));
			let client = tangram_http::client::Builder::new(addr).build();
			client.connect().await.unwrap();

			let state = State {
				packages: BTreeMap::new(),
				dependencies: HashMap::new(),
			};

			Self {
				client: Arc::new(client),
				state: Arc::new(Mutex::new(state)),
			}
		}

		pub fn publish(&self, metadata: tg::package::Metadata, artifact: tg::Artifact) {
			assert!(metadata.name.is_some());
			assert!(metadata.version.is_some());
			let mut state = self.state.lock().unwrap();
			let entry = state
				.packages
				.entry(metadata.name.as_ref().unwrap().clone())
				.or_default();
			let existing = entry.iter().any(|package| package.metadata == metadata);
			assert!(!existing);
			entry.push(MockPackage { metadata, artifact });
		}

		pub async fn create_mock_package(
			&self,
			name: &str,
			version: &str,
			dependencies: &[tg::Dependency],
		) {
			let imports = dependencies
				.iter()
				.map(|dep| {
					format!(
						r#"import * as {} from "tangram:{}@{}";"#,
						dep.name.as_ref().unwrap(),
						dep.name.as_ref().unwrap(),
						dep.version.as_ref().unwrap()
					)
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

				export default tg.target(() => "Hello, from {name}!");
			"#
			);

			let contents = tg::blob::Blob::with_reader(self.client.as_ref(), contents.as_bytes())
				.await
				.unwrap();
			let tangram_tg = tg::Artifact::from(tg::File::builder(contents).build());
			let artifact = tg::Directory::with_object(tg::directory::Object {
				entries: [(ROOT_MODULE_FILE_NAME.to_owned(), tangram_tg)]
					.into_iter()
					.collect(),
			})
			.into();

			let metadata = tg::package::Metadata {
				name: Some(name.into()),
				version: Some(version.into()),
				description: None,
			};

			{
				let mut state = self.state.lock().unwrap();
				state
					.dependencies
					.insert(metadata.clone(), dependencies.to_vec());
			}
			self.publish(metadata, artifact);
		}

		pub async fn try_solve(
			&self,
			metadata: tg::package::Metadata,
		) -> tangram_error::Result<Vec<(tg::Path, tg::Lock)>> {
			let package: tg::Id = self
				.get_package_version(
					metadata.name.as_ref().unwrap(),
					metadata.version.as_ref().unwrap(),
				)
				.await
				.unwrap()
				.unwrap()
				.into();
			let registry_dependencies = self.registry_dependencies(&[package.clone()]).await;
			solve(self, package, BTreeMap::new(), registry_dependencies).await
		}

		pub async fn registry_dependencies(
			&self,
			packages: &[tg::Id],
		) -> BTreeSet<(tg::Id, tg::Dependency)> {
			let mut set = BTreeSet::new();
			for package in packages {
				let dependencies = self
					.get_package_dependencies(package)
					.await
					.expect("Failed to get the package dependencies.")
					.into_iter()
					.flatten()
					.filter_map(|d| d.path.is_none().then_some((package.clone(), d)));
				set.extend(dependencies);
			}
			set
		}
	}

	#[async_trait]
	impl tg::Client for MockClient {
		fn clone_box(&self) -> Box<dyn tg::Client> {
			Box::new(self.clone())
		}

		fn path(&self) -> Option<&Path> {
			self.client.path()
		}

		fn file_descriptor_semaphore(&self) -> &tokio::sync::Semaphore {
			self.client.file_descriptor_semaphore()
		}

		async fn stop(&self) -> tangram_error::Result<()> {
			self.client.stop().await
		}

		async fn status(&self) -> tangram_error::Result<tg::status::Status> {
			self.client.status().await
		}

		async fn clean(&self) -> tangram_error::Result<()> {
			self.client.clean().await
		}

		async fn get_object_exists(&self, id: &tg::object::Id) -> tangram_error::Result<bool> {
			self.client.get_object_exists(id).await
		}

		async fn try_get_object(
			&self,
			id: &tg::object::Id,
		) -> tangram_error::Result<Option<bytes::Bytes>> {
			self.client.try_get_object(id).await
		}

		async fn try_put_object(
			&self,
			id: &tg::object::Id,
			bytes: &bytes::Bytes,
		) -> tangram_error::Result<tangram_error::Result<(), Vec<tg::object::Id>>> {
			self.client.try_put_object(id, bytes).await
		}

		async fn try_get_tracker(&self, path: &Path) -> tangram_error::Result<Option<tg::Tracker>> {
			self.client.try_get_tracker(path).await
		}

		async fn set_tracker(
			&self,
			path: &Path,
			tracker: &tg::Tracker,
		) -> tangram_error::Result<()> {
			self.client.set_tracker(path, tracker).await
		}

		async fn try_get_build_for_target(
			&self,
			id: &tg::target::Id,
		) -> tangram_error::Result<Option<tg::build::Id>> {
			self.client.try_get_build_for_target(id).await
		}

		async fn get_or_create_build_for_target(
			&self,
			id: &tg::target::Id,
		) -> tangram_error::Result<tg::build::Id> {
			self.client.get_or_create_build_for_target(id).await
		}

		async fn try_get_build_target(
			&self,
			id: &tg::build::Id,
		) -> tangram_error::Result<Option<tg::target::Id>> {
			self.client.try_get_build_target(id).await
		}

		async fn try_get_build_children(
			&self,
			id: &tg::build::Id,
		) -> tangram_error::Result<Option<BoxStream<'static, tangram_error::Result<tg::build::Id>>>>
		{
			self.client.try_get_build_children(id).await
		}

		async fn add_build_child(
			&self,
			build_id: &tg::build::Id,
			child_id: &tg::build::Id,
		) -> tangram_error::Result<()> {
			self.client.add_build_child(build_id, child_id).await
		}

		async fn try_get_build_log(
			&self,
			id: &tg::build::Id,
		) -> tangram_error::Result<Option<BoxStream<'static, tangram_error::Result<bytes::Bytes>>>>
		{
			self.client.try_get_build_log(id).await
		}

		async fn add_build_log(
			&self,
			build_id: &tg::build::Id,
			bytes: bytes::Bytes,
		) -> tangram_error::Result<()> {
			self.client.add_build_log(build_id, bytes).await
		}

		async fn try_get_build_result(
			&self,
			id: &tg::build::Id,
		) -> tangram_error::Result<Option<tangram_error::Result<tg::Value>>> {
			self.client.try_get_build_result(id).await
		}

		async fn finish_build(
			&self,
			id: &tg::build::Id,
			result: tangram_error::Result<tg::Value>,
		) -> tangram_error::Result<()> {
			self.client.finish_build(id, result).await
		}

		async fn create_login(&self) -> tangram_error::Result<tg::user::Login> {
			self.client.create_login().await
		}

		async fn get_login(&self, id: &tg::Id) -> tangram_error::Result<Option<tg::user::Login>> {
			self.client.get_login(id).await
		}

		async fn get_current_user(
			&self,
			token: &str,
		) -> tangram_error::Result<Option<tg::user::User>> {
			self.client.get_current_user(token).await
		}

		async fn search_packages(
			&self,
			quer: &str,
		) -> tangram_error::Result<Vec<tg::package::Package>> {
			let Some(mock) = self.get_package(quer).await? else {
				return Ok(vec![]);
			};
			Ok(vec![mock])
		}

		async fn get_package(
			&self,
			name: &str,
		) -> tangram_error::Result<Option<tg::package::Package>> {
			let state = self.state.lock().unwrap();
			let Some(mock) = state.packages.get(name) else {
				return Ok(None);
			};

			let versions = mock
				.iter()
				.map(|mock| mock.metadata.version.clone().unwrap())
				.collect();
			Ok(Some(tg::Package {
				name: name.into(),
				versions,
			}))
		}

		async fn get_package_version(
			&self,
			name: &str,
			version: &str,
		) -> tangram_error::Result<Option<tg::artifact::Id>> {
			let artifact = {
				let state = self.state.lock().unwrap();
				let Some(mock) = state.packages.get(name) else {
					return Ok(None);
				};

				let Some(artifact) = mock.iter().find_map(|mock| {
					(mock.metadata.version.as_deref().unwrap() == version)
						.then_some(mock.artifact.clone())
				}) else {
					return Ok(None);
				};
				artifact
			};

			let id = artifact.id(self.client.as_ref()).await?;
			Ok(Some(id))
		}

		async fn publish_package(
			&self,
			_token: &str,
			id: &tg::artifact::Id,
		) -> tangram_error::Result<()> {
			let directory = tg::Artifact::with_id(id.clone())
				.try_unwrap_directory()
				.wrap_err("Failed to get directory")?;
			let text = directory
				.try_get(
					self.client.as_ref(),
					&ROOT_MODULE_FILE_NAME.parse().unwrap(),
				)
				.await?
				.unwrap()
				.try_unwrap_file()
				.wrap_err("Failed to get the file.")?
				.contents(self.client.as_ref())
				.await?
				.text(self.client.as_ref())
				.await?;
			let module = crate::Module::analyze(text)?;
			let metadata = module.metadata.unwrap();
			self.publish(metadata, tg::Artifact::with_id(id.clone()));
			Ok(())
		}

		async fn get_package_metadata(
			&self,
			id: &tg::Id,
		) -> tangram_error::Result<Option<tg::package::Metadata>> {
			self.client.get_package_metadata(id).await
		}

		async fn get_package_dependencies(
			&self,
			id: &tg::Id,
		) -> tangram_error::Result<Option<Vec<tg::Dependency>>> {
			self.client.get_package_dependencies(id).await
		}

		async fn get_build_from_queue(&self) -> tangram_error::Result<tg::build::Id> {
			self.client.get_build_from_queue().await
		}

		async fn cancel_build(&self, id: &tg::build::Id) -> tangram_error::Result<()> {
			self.client.cancel_build(id).await
		}
	}
}
