use super::Compiler;
use crate::{
	hash::Hash,
	js::compiler::resolve::{TANGRAM_MODULE_SCHEME, TANGRAM_TARGET_SCHEME},
};
use anyhow::{bail, ensure, Context, Result};
use camino::Utf8Path;
use indoc::writedoc;
use std::fmt::Write;
use tokio::io::AsyncReadExt;
use url::Url;

impl Compiler {
	pub async fn load(&self, url: Url) -> Result<String> {
		match url.scheme() {
			// Load a module with the tangram module scheme.
			TANGRAM_MODULE_SCHEME => self.load_module(url).await,

			// Load a module with the tangram target proxy scheme.
			TANGRAM_TARGET_SCHEME => self.load_target(url).await,

			_ => {
				bail!(r#"The URL "{url}" has an unsupported scheme."#);
			},
		}
	}

	async fn load_module(&self, url: Url) -> Result<String> {
		// Lock the builder.
		let builder = self.state.builder.lock_shared().await?;

		// Ensure the url has the tangram module scheme.
		ensure!(
			url.scheme() == TANGRAM_MODULE_SCHEME,
			r#"The URL "{url}" must have the scheme "{TANGRAM_MODULE_SCHEME}"."#,
		);

		// Get the package hash from the url.
		let domain = url.domain().context("The URL must have a domain.")?;
		let package_hash: Hash = domain.parse()?;

		// Get the path from the url.
		let path = Utf8Path::new(url.path());
		let path = path
			.strip_prefix("/")
			.with_context(|| "The URL must have a leading slash.")?;

		// Find the module in the package.
		let package_source_hash = builder
			.get_package_source(package_hash)
			.context("Failed to get the package source.")?;
		let package_source_artifact = builder
			.get_expression_local(package_source_hash)?
			.into_artifact()
			.context("Expected the package source to be an artifact.")?;
		let mut expression = builder.get_expression_local(package_source_artifact.root)?;
		for component in path.components() {
			expression = builder.get_expression_local(
				expression
					.into_directory()
					.context("Expected a directory.")?
					.entries
					.get(component.as_str())
					.copied()
					.context(r#"Failed to find file at path {path}"#)?,
			)?;
		}

		// Read the module.
		let file = expression.into_file().context("Expected a file.")?;
		let mut source = String::new();
		builder
			.get_blob(file.blob)
			.await?
			.read_to_string(&mut source)
			.await?;

		Ok(source)
	}

	async fn load_target(&self, url: Url) -> Result<String> {
		// Lock the builder.
		let builder = self.state.builder.lock_shared().await?;

		// Ensure the specifier has the tangram target proxy scheme.
		ensure!(
			url.scheme() == TANGRAM_TARGET_SCHEME,
			r#"The URL "{url}" must have the scheme "{TANGRAM_TARGET_SCHEME}"."#,
		);

		// Get the package from the specifier.
		let domain = url.domain().context("The specifier must have a domain.")?;
		let package_hash: Hash = domain
			.parse()
			.context("Failed to parse the domain as a hash.")?;

		// Get the package's manifest.
		let manifest = builder
			.get_package_manifest(package_hash)
			.await
			.context("Failed to get the package manifest.")?;

		// Generate the code for the target proxy module.
		let mut code = String::new();
		writedoc!(
			code,
			r#"let _package = await Tangram.getExpression(new Tangram.Hash("{package_hash}"));"#
		)
		.unwrap();
		code.push('\n');
		for target_name in manifest.targets {
			if target_name == "default" {
				writedoc!(code, r#"export default "#).unwrap();
			} else {
				writedoc!(code, r#"export let {target_name} = "#).unwrap();
			}
			writedoc!(
				code,
				r#"
				(...args) => new Tangram.Target({{
					package: _package,
					name: "{target_name}",
					args,
				}});
			"#,
			)
			.unwrap();
			code.push('\n');
		}

		Ok(code)
	}
}
