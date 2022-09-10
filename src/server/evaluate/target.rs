use crate::{
	expression::{self, Expression},
	server::Server,
	util::path_exists,
};
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
	) -> Result<Expression> {
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

		// Create the JS process expression.
		let expression =
			expression::Expression::Process(expression::Process::Js(expression::JsProcess {
				lockfile: target.lockfile.clone(),
				module: Box::new(expression::Expression::Path(expression::Path {
					artifact: Box::new(expression::Expression::Artifact(target.package)),
					path,
				})),
				export: target.name.clone(),
				args: target.args.clone(),
			}));

		// Evaluate the expression.
		let output = self.evaluate(&expression).await?;

		Ok(output)
	}
}
