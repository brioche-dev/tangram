use crate::{
	expression::{self, Expression},
	hash::Hash,
	server::{Evaluator, Server},
	util::path_exists,
};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use camino::Utf8PathBuf;
use std::sync::Arc;

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
		server: &Arc<Server>,
		hash: Hash,
		expression: &Expression,
	) -> Result<Option<Hash>> {
		let target = if let Expression::Target(target) = expression {
			target
		} else {
			return Ok(None);
		};
		// Create a fragment for the package.
		let package_fragment = server
			.create_fragment(target.package)
			.await
			.context("Failed to create the package artifact.")?;
		let package_fragment_path = server.fragment_path(&package_fragment);

		// Check if the package contains a tangram.js or tangram.ts file.
		let path = if path_exists(&package_fragment_path.join("tangram.js")).await? {
			Some(Utf8PathBuf::from("tangram.js"))
		} else if path_exists(&package_fragment_path.join("tangram.ts")).await? {
			Some(Utf8PathBuf::from("tangram.ts"))
		} else {
			bail!("The package does not contain a tangram.js or tangram.ts.");
		};

		// Add the expressions.
		let artifact_hash = server
			.add_expression(&expression::Expression::Artifact(target.package))
			.await?;
		let module_hash = server
			.add_expression(&expression::Expression::Path(expression::Path {
				artifact: artifact_hash,
				path: path.map(Into::into),
			}))
			.await?;
		let expression_hash = server
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
		let output = server.evaluate(expression_hash, hash).await?;

		Ok(Some(output))
	}
}
