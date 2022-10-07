use crate::{
	builder,
	evaluators::Evaluator,
	expression::{self, Expression},
	hash::Hash,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use camino::Utf8PathBuf;

pub struct Target;

impl Target {
	#[must_use]
	pub fn new() -> Target {
		Target {}
	}
}

impl Default for Target {
	fn default() -> Self {
		Target::new()
	}
}

#[async_trait]
impl Evaluator for Target {
	async fn evaluate(
		&self,
		builder: &builder::Shared,
		hash: Hash,
		expression: &Expression,
	) -> Result<Option<Hash>> {
		let target = if let Expression::Target(target) = expression {
			target
		} else {
			return Ok(None);
		};

		// Get the package's dependencies.
		let package = builder
			.get_expression(target.package)
			.await?
			.into_package()
			.ok_or_else(|| anyhow!("Expected a package expression."))?;

		// Resolve the package's entry point.
		let entrypoint = builder
			.resolve_package_entrypoint_file(&package)
			.await
			.context("Could not resolve entrypoint to package")?
			.context("Package did not have an entry point file")?;

		// Add the js process expression.
		let expression_hash = builder
			.add_expression(&expression::Expression::Js(expression::Js {
				dependencies: package.dependencies,
				artifact: target.package,
				path: entrypoint,
				name: target.name.clone(),
				args: target.args,
			}))
			.await?;

		// Evaluate the expression.
		let output = builder.evaluate(expression_hash, hash).await?;

		Ok(Some(output))
	}
}

/// List of candidate filenames, in priority order, for resolving the script
/// entrypoint to a Tangram package.
const CANDIDATE_ENTRYPOINT_FILENAMES: &[&str] = &["tangram.ts", "tangram.js"];

impl builder::Shared {
	/// Given a package expression, resolve the filename of the script entry point.
	///
	/// See [`CANDIDATE_ENTRYPOINT_FILENAMES`] for an ordered list of the filenames this function
	/// will check for in the package root.
	///
	/// If no suitable file is found, returns `None`.
	pub async fn resolve_package_entrypoint_file(
		&self,
		package: &expression::Package,
	) -> Result<Option<Utf8PathBuf>> {
		// Get the root package artifact.
		let source_artifact: expression::Artifact = self
			.get_expression(package.source)
			.await
			.context("Failed to get package source")?
			.into_artifact()
			.context("Package source was not an artifact expression")?;

		let source_directory: expression::Directory = self
			.get_expression(source_artifact.root)
			.await
			.context("Failed to get contents of package source artifact")?
			.into_directory()
			.context("Package source artifact did not contain a directory")?;

		// Look through the list of candidates, returning the first one which matches.
		for candidate in CANDIDATE_ENTRYPOINT_FILENAMES {
			if source_directory.entries.contains_key(candidate as &str) {
				return Ok(Some(candidate.into()));
			}
		}

		// Here, we've fallen through the candidates list, and there's no suitable entrypoint file.
		Ok(None)
	}
}
