use async_recursion::async_recursion;
use core::fmt;
use im::HashMap;
use std::collections::BTreeMap;
use tangram_client as tg;
use tg::{Client, WrapErr};

use crate::Analysis;

/// Errors that may arise during version solving.
#[derive(Debug, Clone)]
pub enum Error {
	/// The package does not exist in the registry.
	PackageOoesNotExist,

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

	/// Semantive version parsing error.
	Semver(String),

	/// A tangram error.
	Other(tg::Error),
}

/// Given a registry and unlocked package, create a lockfile for it. If no solution can be found, a [Report] containing a description of the most recent set of errors is returned.
pub async fn solve(
	client: &dyn Client,
	root: tg::Id,
	path_dependencies: BTreeMap<tg::Id, BTreeMap<tg::Relpath, tg::Id>>,
) -> tg::Result<Result<tg::Lock, Report>> {
	// Create the context.
	let mut context = Context::new(client, path_dependencies);

	// Solve.
	let solution = solve_inner(&mut context, root.clone()).await?;

	// Create the error report.
	let errors = solution
		.partial
		.iter()
		.filter_map(|(dependent, partial)| match partial {
			Mark::Permanent(Err(e)) => Some((dependent.clone(), e.clone())),
			_ => None,
		})
		.collect::<Vec<_>>();

	// If the report is not empty, return an error.
	if !errors.is_empty() {
		return Ok(Err(Report {
			errors,
			context,
			solution,
		}));
	}

	// Now we have the solution, create a lock.
	let lock = lock(&context, &solution, root).await?;
	Ok(Ok(lock))
}

#[async_recursion]
async fn lock(context: &Context, solution: &Solution, package: tg::Id) -> tg::Result<tg::Lock> {
	// Retrieve the dependencies for the package. The unwrap() is safe since we cannot reach this point if a package has not been added to the context.
	let dependencies = &context.analysis.get(&package).unwrap().dependencies;

	// Lock each dependency.
	let mut locked_dependencies = BTreeMap::default();
	for dependency in dependencies {
		let dependant = Dependant {
			package: package.clone(),
			dependency: dependency.clone(),
		};

		// The only way for a temporary mark to remain is if the solving algorithm is implemented incorrectly.
		let Mark::Permanent(Ok(package)) = solution.partial.get(&dependant).cloned().unwrap()
		else {
			tg::return_error!("Internal error, solution is incomplete.");
		};

		let lock = lock(context, solution, package.clone()).await?;
		let id = tg::artifact::Id::try_from(package)
			.wrap_err("Failed to get package artifact from its id.")?;
		let package = tg::Artifact::with_id(id.clone());
		let entry = tg::lock::Entry { lock, package };
		locked_dependencies.insert(dependency.clone(), entry);
	}

	// Create the lock and return.
	let lock = tg::Lock::with_object(tg::lock::Object {
		dependencies: locked_dependencies,
	});
	Ok(lock)
}

async fn solve_inner(context: &mut Context, root: tg::Id) -> tg::Result<Solution> {
	// Create the working set of unsolved dependencies.
	let working_set = context
		.dependencies(&root)
		.await?
		.iter()
		.map(|dependency| Dependant {
			package: root.clone(),
			dependency: dependency.clone(),
		})
		.collect();

	// Create the first stack frame.
	let solution = Solution::empty();
	let last_error = None;
	let remaining_versions = None;
	let mut current_frame = Frame {
		working_set,
		solution,
		last_error,
		remaining_versions,
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

		let permanent = current_frame
			.solution
			.permanent
			.get(dependant.dependency.name.as_ref().unwrap());

		let partial = current_frame.solution.partial.get(&dependant);
		match (permanent, partial) {
			// Case 0: There is no solution for this package yet.
			(None, None) => 'a: {
				tracing::debug!(?dependant, "Creating initial version selection.");

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
							current_frame
								.solution
								.mark_permanently(dependant, Err(Error::Other(e)));

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
						&dependant.package,
						&dependant.dependency,
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
							next_frame.solution.mark_permanently(dependant, Err(e));
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
								next_frame.solution = next_frame
									.solution
									.mark_permanently(dependant, Ok(package.clone()));
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
									next_frame.solution =
										next_frame.solution.mark_permanently(dependant, Err(error));
								}
							},
							Err(e) => {
								tracing::error!(?dependant, ?e, "Existing solution is an error.");
								next_frame
									.solution
									.mark_permanently(dependant, Err(Error::Other(e)));
							},
						}
					},
					// Case 1.2: The less happy path. We know there's no solution to this package because we've already failed to satisfy some other set of constraints.
					Err(e) => {
						next_frame.solution = next_frame
							.solution
							.mark_permanently(dependant, Err(e.clone()));
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
					next_frame.solution = next_frame
						.solution
						.mark_permanently(dependant, Ok(package.clone()));
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
							next_frame.solution.mark_permanently(dependant, Err(error));
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

#[derive(Debug)]
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
	path_dependencies: BTreeMap<tg::Id, BTreeMap<tg::Relpath, tg::Id>>,
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
		path_dependencies: BTreeMap<tg::Id, BTreeMap<tg::Relpath, tg::Id>>,
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

	pub fn add_root(&mut self, package: tg::Id) {
		self.roots.push(package);
	}

	// Check if a package satisfies a dependency.
	fn matches(&self, version: &str, dependency: &tg::Dependency) -> tg::Result<bool> {
		let Some(constraint) = dependency.version.as_ref() else {
			return Ok(true);
		};
		let version: semver::Version = version.parse().map_err(|e| {
			tracing::error!(?e, ?version, "Failed to parse metadata version.");
			tg::error!("Failed to parse version: {version}.")
		})?;
		let constraint: semver::VersionReq = constraint.parse().map_err(|e| {
			tracing::error!(?e, ?dependency, "Failed to parse dependency version.");
			tg::error!("Failed to parse version.")
		})?;

		Ok(constraint.matches(&version))
	}

	// Try and get the next version from a list of remaining ones. Returns an error if the list is empty.
	async fn try_resolve(
		&mut self,
		package: &tg::Id,
		dependency: &tg::Dependency,
		remaining_versions: &mut im::Vector<String>,
	) -> Result<tg::Id, Error> {
		debug_assert!(
			dependency.name.is_some() && dependency.version.is_some() && dependency.path.is_none(),
			"try_get_version is only meaningful for registry dependencies."
		);
		let name = dependency.name.as_ref().unwrap();

		// First check if we have a path dependency table for this package and this is a path dependency. If we cannot look up the path dependency we return an error.
		if let (Some(path_dependencies), Some(path)) = (
			self.path_dependencies.get(package),
			dependency.path.as_ref(),
		) {
			return path_dependencies
				.get(path)
				.cloned()
				.ok_or(Error::Other(tg::error!(
					"Could not find path dependency for {dependency}."
				)));
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
				Err(e) => return Err(Error::Other(e)),
				Ok(Some(package)) => {
					let package: tg::Id = package.into();
					self.published_packages.insert(metadata, package.clone());
					return Ok(package);
				},
				Ok(None) => continue,
			}
		}
	}

	pub async fn analysis(&mut self, package: &tg::Id) -> tg::Result<&'_ Analysis> {
		if !self.analysis.contains_key(package) {
			let metadata = self
				.client
				.get_package_metadata(package)
				.await?
				.ok_or(tg::error!("Missing package metadata."))?;
			let dependencies = self
				.client
				.get_package_dependencies(package)
				.await?
				.into_iter()
				.flatten()
				.filter(|dependency| dependency.path.is_none())
				.collect();
			let analysis = Analysis {
				metadata: metadata.clone(),
				dependencies,
			};
			let _ = self
				.published_packages
				.insert(metadata.clone(), package.clone());
			let _ = self.analysis.insert(package.clone(), analysis);
		}
		Ok(self.analysis.get(package).unwrap())
	}

	// Get a list of registry dependencies for a package given its metadata.
	pub async fn dependencies(&mut self, package: &tg::Id) -> tg::Result<&'_ [tg::Dependency]> {
		Ok(&self.analysis(package).await?.dependencies)
	}

	pub async fn version(&mut self, package: &tg::Id) -> tg::Result<&str> {
		self.analysis(package).await?.version()
	}

	// Lookup all the published versions of a package by name.
	async fn lookup(&mut self, package_name: &str) -> tg::Result<Vec<tg::package::Metadata>> {
		let metadata = self
			.client
			.get_package(package_name)
			.await?
			.ok_or(tg::error!("Could not find package named {package_name}."))?
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

	/// Mark this dependant with a temporary solution.
	fn mark_temporarily(&self, dependant: Dependant, package: tg::Id) -> Self {
		let mut solution = self.clone();
		solution.partial.insert(dependant, Mark::Temporary(package));
		solution
	}

	/// Mark the dependant permanently, adding it to the list of known solutions and the partial solutions.
	fn mark_permanently(&self, dependant: Dependant, complete: Result<tg::Id, Error>) -> Self {
		let mut solution = self.clone();

		// Update the global solution.
		let _old = solution
			.permanent
			.insert(dependant.dependency.name.clone().unwrap(), complete.clone());

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
		for (dependent, error) in &self.errors {
			self.format(f, dependent, error)?;
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
			Error::PackageOoesNotExist => {
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
				let shared_dependents = self
					.solution
					.partial
					.keys()
					.filter(|dependent| dependent.dependency.name == dependency.name);
				for shared in shared_dependents {
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
					let dependent = Dependant {
						package: package.clone(),
						dependency: child.clone(),
					};
					self.format(f, &dependent, error)?;
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

pub async fn path_overrides(
	client: &dyn Client,
	root: tg::Directory,
	resolver: impl Fn(tg::Directory, &tg::Relpath) -> Option<tg::Directory>,
) -> tg::Result<HashMap<String, tg::Id>> {
	let mut table = HashMap::new();
	let mut stack = vec![root];
	while let Some(next) = stack.pop() {
		let package = next.id(client).await?.clone().into();
		let Ok(Some(dependencies)) = client.get_package_dependencies(&package).await else {
			continue;
		};
		let Ok(Some(metadata)) = client.get_package_metadata(&package).await else {
			continue;
		};
		for dependency in dependencies {
			match (&dependency.name, &metadata.name) {
				(Some(dependency_name), Some(package_name)) if package_name == dependency_name => {
					table.insert(dependency_name.clone(), package.clone());
				},
				_ => (),
			}
			if let Some(path) = dependency.path.as_ref() {
				if let Some(package) = resolver(next.clone(), path) {
					stack.push(package);
				}
			}
		}
	}

	Ok(table)
}
