use crate::{
	builder::Shared,
	expression::{self, Target},
	hash::Hash,
};
use anyhow::{Context, Result};
use camino::Utf8PathBuf;

impl Shared {
	pub(super) async fn evaluate_target(&self, hash: Hash, target: &Target) -> Result<Hash> {
		// Get the path to the package's JS module.
		let path = self
			.get_package_js_path(target.package)
			.await
			.context("Failed to resolve the entrypoint to the package.")?
			.context("The package did not have an entry point file.")?;

		// Add the js process expression.
		let expression_hash = self
			.add_expression(&expression::Expression::Js(expression::Js {
				package: target.package,
				path,
				name: target.name.clone(),
				args: target.args,
			}))
			.await?;

		// Evaluate the expression.
		let output = self.evaluate(expression_hash, hash).await?;

		Ok(output)
	}
}

const CANDIDATE_ENTRYPOINT_FILENAMES: &[&str] = &["tangram.ts", "tangram.js"];

impl Shared {
	pub async fn get_package_js_path(&self, hash: Hash) -> Result<Option<Utf8PathBuf>> {
		// Get the package.
		let package = self
			.get_expression(hash)
			.await?
			.into_package()
			.context("Expected a package expression.")?;

		// Get the root package artifact.
		let source_artifact: expression::Artifact = self
			.get_expression(package.source)
			.await
			.context("Failed to get the package source.")?
			.into_artifact()
			.context("The package source must be an artifact expression.")?;

		// Get the source directory.
		let source_directory: expression::Directory = self
			.get_expression(source_artifact.root)
			.await
			.context("Failed to get the contents of the package source artifact.")?
			.into_directory()
			.context("The package source artifact did not contain a directory.")?;

		for candidate in CANDIDATE_ENTRYPOINT_FILENAMES {
			if source_directory.entries.contains_key(candidate as &str) {
				return Ok(Some(candidate.into()));
			}
		}

		Ok(None)
	}
}
