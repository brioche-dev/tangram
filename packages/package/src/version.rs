use core::fmt;
use std::collections::BTreeMap;

use async_recursion::async_recursion;
use im::HashMap;
// use std::path::PathBuf;
use tangram_client as tg;
use tg::{Client, WrapErr};

use crate::{scan_direct_dependencies, Analysis};

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
	root: tg::package::Metadata,
	dependencies: Vec<tg::Dependency>,
) -> Result<tg::Lock, Report> {
	// Create the input to the solver.
	let mut context = Context::new(client);

	let root = Unsolved {
		metadata: root,
		dependencies,
	};

	// Solve.
	let solution = solve_inner(&mut context, root.clone()).await;

	// Check for errors and bail if any are detected.
	let errors = solution
		.partial
		.iter()
		.filter_map(|(dependent, partial)| match partial {
			Mark::Permanent(Err(e)) => Some((dependent.clone(), e.clone())),
			_ => None,
		})
		.collect::<Vec<_>>();

	if !errors.is_empty() {
		return Err(Report { errors, solution });
	}

	// Now we have the solution, create a lock.
	let lock = lock(&mut context, &solution, root).await.unwrap();
	Ok(lock)
}

#[async_recursion]
async fn lock(
	context: &mut Context,
	solution: &Solution,
	unlocked: Unsolved,
) -> tg::Result<tg::Lock> {
	let mut locked_dependencies = BTreeMap::default();
	for dependency in unlocked.dependencies {
		let dependant = Dependant {
			metadata: unlocked.metadata.clone(),
			dependency: dependency.clone(),
		};

		let Mark::Permanent(Ok(version)) = solution.partial.get(&dependant).cloned().unwrap()
		else {
			tg::return_error!("Internal error, solution is incomplete.");
		};

		let metadata = tg::package::Metadata {
			name: dependency.name.clone(),
			version: Some(version),
			description: None,
		};

		let dependencies = context.dependencies(&metadata).await?.to_vec();

		let package = context.artifact(&metadata).await?;
		let unlocked = Unsolved {
			metadata,
			dependencies,
		};

		let lock = lock(context, solution, unlocked).await?;
		let entry = tg::lock::Entry { lock, package };
		locked_dependencies.insert(dependency, entry);
	}

	let lock = tg::Lock::with_object(tg::lock::Object {
		dependencies: locked_dependencies,
	});
	Ok(lock)
}

async fn solve_inner(context: &mut Context, root: Unsolved) -> Solution {
	let dependencies = root
		.dependencies
		.into_iter()
		.map(|d| Dependant {
			metadata: root.metadata.clone(),
			dependency: d,
		})
		.collect();
	let mut history = im::Vector::new();

	let mut current_frame = Frame {
		working_set: dependencies,
		solution: Solution::empty(),
		last_error: None,
		remaining_versions: None,
	};

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
					let all_versions = match context.lookup(dependant.dependency.name.as_ref().unwrap()).await {
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
				let version =
					context.try_get_version(current_frame.remaining_versions.as_mut().unwrap());

				match version {
					// We successfully popped a version.
					Some(version) => {
						next_frame.solution = next_frame
							.solution
							.mark_temporarily(dependant.clone(), version.clone());

						// Add this dependency to the top of the stack before adding all its dependencies.
						next_frame.working_set.push_back(dependant.clone());

						// Add all the dependencies to the stack.
						let metadata = tg::package::Metadata {
							name: dependant.dependency.name.clone(),
							version: Some(version.clone()),
							description: None,
						};

						for child_dependency in context.dependencies(&metadata).await.unwrap() {
							let dependant = Dependant {
								metadata: metadata.clone(),
								dependency: child_dependency.clone(),
							};
							next_frame.working_set.push_back(dependant);
						}

						// Update the solution
						next_frame.solution =
							next_frame.solution.mark_temporarily(dependant, version);

						// Update the stack. If we backtrack, we use the next version in the version stack.
						history.push_back(current_frame.clone());
					},

					None => {
						tracing::error!(?dependant, "No solution exists.");
						let error = current_frame
							.last_error
							.clone()
							.unwrap_or(Error::PackageVersionConflict);
						next_frame.solution =
							next_frame.solution.mark_permanently(dependant, Err(error));
					},
				}
			},

			// Case 1: There exists a global version for the package but we haven't solved this dependency constraint.
			(Some(permanent), None) => {
				tracing::debug!(?dependant, ?permanent, "Existing solution found.");
				match permanent {
					// Case 1.1: The happy path. Our version is solved and it matches this constraint.
					Ok(version) => {
						// Case 1.1: The happy path. Our version is solved and it matches this constraint.
						match context.matches(version, &dependant.dependency) {
							Ok(true) => {
								next_frame.solution = next_frame
									.solution
									.mark_permanently(dependant, Ok(version.clone()));
							},
							// Case 1.3: The unhappy path. We need to fail.
							Ok(false) => {
								tracing::warn!(
									?dependant,
									?version,
									"Package version conflict detected."
								);
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
			(_, Some(Mark::Temporary(version))) => {
				let metadata = tg::package::Metadata {
					name: dependant.dependency.name.clone(),
					version: Some(version.clone()),
					description: None,
				};

				let dependencies = context.dependencies(&metadata).await.unwrap();

				let mut erroneous_children = vec![];

				for child_dependency in dependencies {
					let child_dependant = Dependant {
						metadata: metadata.clone(),
						dependency: child_dependency.clone(),
					};

					let child = next_frame.solution.partial.get(&child_dependant).unwrap();
					match child {
						Mark::Permanent(Ok(_)) => (),
						Mark::Permanent(Err(e)) => {
							let error = e.clone();
							erroneous_children.push((child_dependant.dependency, error));
						},
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
						.mark_permanently(dependant, Ok(version.clone()));
				} else {
					let error = Error::BacktrackError {
						previous_version: version.clone(),
						erroneous_dependencies: erroneous_children,
					};

					if let Some(frame_) =
						try_backtrack(&history, dependant.dependency.name.as_ref().unwrap(), error.clone())
					{
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

	current_frame.solution
}

pub struct Context {
	client: Box<dyn tg::Client>,
	cache: HashMap<tg::package::Metadata, Vec<tg::Dependency>>,
	cache2: HashMap<String, HashMap<String, Vec<Analysis>>>,
}

/// The Report is an error type that can be pretty printed to describe why version solving failed.
#[derive(Debug)]
pub struct Report {
	errors: Vec<(Dependant, Error)>,
	solution: Solution,
}

#[derive(Clone, Debug)]
struct Unsolved {
	metadata: tg::package::Metadata,
	dependencies: Vec<tg::Dependency>,
}

#[derive(Clone, Debug)]
struct Solution {
	permanent: im::HashMap<String, Result<String, Error>>,
	partial: im::HashMap<Dependant, Mark>,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Dependant {
	metadata: tg::package::Metadata,
	dependency: tg::Dependency,
}

#[derive(Clone, Debug)]
enum Mark {
	Temporary(String),
	Permanent(Result<String, Error>),
}

#[derive(Clone, Debug)]
struct Frame {
	solution: Solution,
	working_set: im::Vector<Dependant>,
	remaining_versions: Option<im::Vector<String>>,
	last_error: Option<Error>,
}

impl Context {
	fn new(client: &dyn tg::Client) -> Self {
		let client = client.clone_box();
		let cache = HashMap::new();
		let cache2 = HashMap::new();
		Self { client, cache, cache2 }
	}

	// Check if a package satisfies a dependency.
	fn matches(&self, version: &str, dependency: &tg::Dependency) -> tg::Result<bool> {
		let Some(constraint) = dependency.version.as_ref() else {
			return Ok(true);
		};
		let version: semver::Version = version.parse().map_err(|e| {
			tracing::error!(?e, ?version, "Failed to parse metadata version.");
			tg::error!("Failed to parse version.")
		})?;
		let constraint: semver::VersionReq = constraint.parse().map_err(|e| {
			tracing::error!(?e, ?dependency, "Failed to parse dependency version.");
			tg::error!("Failed to parse version.")
		})?;

		Ok(constraint.matches(&version))
	}

	// Try and get the next version from a list of remaining ones. Returns an error if the list is empty.
	fn try_get_version(&self, remaining_versions: &mut im::Vector<String>) -> Option<String> {
		remaining_versions.pop_back().clone()
	}

	// Get a list of registry dependencies for a package given its metadata.
	async fn dependencies(
		&mut self,
		metadata: &tg::package::Metadata,
	) -> tg::Result<&'_ [tg::Dependency]> {
		if !self.cache.contains_key(metadata) {
			let dependencies = self.scan_registry_dependencies(metadata).await?;
			self.cache.insert(metadata.clone(), dependencies);
		}
		Ok(self.cache.get(metadata).unwrap())
	}

	async fn get_or_insert_package (&self, metadata: &tg::package::Metadata) -> tg::Result<()> {
		todo!()
		// let name = metadata.name.as_ref().unwrap();
		// let version = metadata.version.as_ref().unwrap();

		// Avoid using the entry api here in order to avoid the allocation.
		// let entry = match self.cache2.get_mut(name) {
		// 	Some(entry) => entry,
		// 	None => {
		// 		self.cache2.insert(name.into(), HashMap::new());
		// 		self.cache2.get_mut(name).unwrap()
		// 	}
		// };
		// todo!()
		// let dependencies = match entry.get(version) {
		// 	Some(dependency) => dependencies,
		// 	None => {
		// 		let package = self.client.get_package_version(name, version)
		// 			.await?
		// 			.ok_or(tg::error!("Package version does not exist."))?;
		// 		let artifact = tg::Artifact::with_id(package);
		// 		let
		// 	}
		// }

		// Ok(())
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
			.collect();
		Ok(metadata)
	}

	// Collect the registry dependencies of a package given its metadata.
	async fn scan_registry_dependencies(
		&self,
		metadata: &tg::package::Metadata,
	) -> tg::Result<Vec<tg::Dependency>> {
		let name = metadata
			.name
			.as_ref()
			.ok_or(tg::error!("Missing name in metadata."))?;
		let version = metadata
			.version
			.as_ref()
			.ok_or(tg::error!("Missing version in metadata."))?;
		let Some(package) = self.client.get_package_version(name, version).await? else {
			tg::return_error!("Could not find package {name}@{version}");
		};

		let artifact = tg::Artifact::with_id(package)
			.try_unwrap_directory()
			.wrap_err("Expected the package artifact to be a directory.")?;

		let dependencies = scan_direct_dependencies(self.client.as_ref(), artifact)
			.await?
			.into_iter()
			.filter(|dependency| {
				dependency.path.is_none()
			})
			.collect();

		Ok(dependencies)
	}

	async fn artifact(&self, metadata: &tg::package::Metadata) -> tg::Result<tg::Artifact> {
		let name = metadata
			.name
			.as_ref()
			.ok_or(tg::error!("Missing name in metadata."))?;
		let version = metadata
			.version
			.as_ref()
			.ok_or(tg::error!("Missing version in metadata."))?;
		let Some(package) = self.client.get_package_version(name, version).await? else {
			tg::return_error!("Could not find package {name}@{version}");
		};
		Ok(tg::Artifact::with_id(package))
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
	fn mark_temporarily(&self, dependent: Dependant, version: String) -> Self {
		let mut solution = self.clone();
		solution.partial.insert(dependent, Mark::Temporary(version));
		solution
	}

	/// Mark the dependant permanently, adding it to the list of known solutions and the partial solutions.
	fn mark_permanently(&self, dependent: Dependant, complete: Result<String, Error>) -> Self {
		let mut solution = self.clone();

		// Update the global solution.
		let _old = solution
			.permanent
			.insert(dependent.dependency.name.clone().unwrap(), complete.clone());

		// Update the local solution.
		let _old = solution
			.partial
			.insert(dependent, Mark::Permanent(complete));

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
		dependent: &Dependant,
		error: &Error,
	) -> fmt::Result {
		let Dependant {
			metadata,
			dependency,
		} = dependent;

		let name = metadata.name.as_ref().unwrap();
		let version = metadata.version.as_ref().unwrap();
		write!(f, "{name} @ {version} requires {dependency}, but ")?;

		match error {
			Error::Semver(semver) => writeln!(f, "there is a semver error: {semver}."),
			Error::PackageOoesNotExist => {
				writeln!(f, "no package by that name exists in the registry.\n")
			},
			Error::PackageCycleExists {
				dependant: first, ..
			} => {
				writeln!(f, "{first}, which creates a cycle.")
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
						metadata,
						dependency,
					} = shared;
					let name = metadata.name.as_ref().unwrap();
					let version = metadata.version.as_ref().unwrap();
					writeln!(f, "{name} @ {version} requires {dependency}")?;
				}
				Ok(())
			},
			Error::BacktrackError {
				previous_version,
				erroneous_dependencies,
			} => {
				writeln!(f, "{} {previous_version} has errors:", dependency.name.as_ref().unwrap())?;
				for (child, error) in erroneous_dependencies {
					let dependent = Dependant {
						metadata: tg::package::Metadata {
							name: Some(dependency.name.clone().unwrap()),
							version: Some(previous_version.clone()),
							description: None,
						},
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

impl fmt::Display for Dependant {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let Dependant {
			metadata,
			dependency,
		} = self;
		let name = metadata.name.as_ref().unwrap();
		let version = metadata.version.as_ref().unwrap();
		write!(f, "{name} @ {version} requires {dependency}")
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
