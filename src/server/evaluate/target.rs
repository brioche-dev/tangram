use crate::{expression, hash::Hash, server::Server, util::path_exists};
use anyhow::{bail, Context, Result};
use async_recursion::async_recursion;
use camino::Utf8PathBuf;
use std::sync::Arc;

impl Server {
	#[allow(clippy::must_use_candidate)]
	#[async_recursion]
	pub async fn evaluate_target(
		self: &Arc<Self>,
		target: &expression::Target,
		parent_hash: Hash,
	) -> Result<Hash> {
		// Create a fragment for the package.
		let package_fragment = self
			.create_fragment(target.package)
			.await
			.context("Failed to create the package artifact.")?;
		let package_fragment_path = self.fragment_path(&package_fragment);

		// Check if the package contains a tangram.js or tangram.ts file.
		let path = if path_exists(&package_fragment_path.join("tangram.js")).await? {
			Some(Utf8PathBuf::from("tangram.js"))
		} else if path_exists(&package_fragment_path.join("tangram.ts")).await? {
			Some(Utf8PathBuf::from("tangram.ts"))
		} else {
			bail!("The package does not contain a tangram.js or tangram.ts.");
		};

		// Add the expressions.
		let hash = self
			.add_expression(&expression::Expression::Artifact(target.package))
			.await?;
		let module_hash = self
			.add_expression(&expression::Expression::Path(expression::Path {
				artifact: hash,
				path: path.map(Into::into),
			}))
			.await?;
		let expression = self
			.add_expression(&expression::Expression::Process(expression::Process::Js(
				expression::JsProcess {
					lockfile: target.lockfile.clone(),
					module: module_hash,
					export: target.name.clone(),
					args: target.args.clone(),
				},
			)))
			.await?;

		// Evaluate the expression.
		let output = self.evaluate(expression, parent_hash).await?;

		Ok(output)
	}
}
