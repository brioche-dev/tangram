use super::Compiler;
use crate::{hash::Hash, js, manifest::Manifest};
use anyhow::{bail, Context, Result};
use camino::Utf8Path;
use indoc::writedoc;
use std::{fmt::Write, path::Path};
use tokio::io::AsyncReadExt;

impl Compiler {
	pub async fn load(&self, url: &js::Url) -> Result<String> {
		match url {
			js::Url::PackageModule {
				package_hash,
				sub_path,
			} => self.load_package_module(*package_hash, sub_path).await,
			js::Url::PackageTargets { package_hash } => {
				self.load_package_targets(*package_hash).await
			},
			js::Url::PathModule {
				package_path: path,
				sub_path,
			} => self.load_path_module(path, sub_path).await,
			js::Url::PathTargets { package_path } => self.load_path_targets(package_path).await,
			js::Url::TsLib => bail!(r#"Cannot load from URL "{url}"."#),
		}
	}

	async fn load_package_module(&self, package_hash: Hash, sub_path: &Utf8Path) -> Result<String> {
		// Lock the builder.
		let builder = self.state.builder.lock_shared().await?;

		// Find the module in the package.
		let package_source_hash = builder
			.get_package_source(package_hash)
			.context("Failed to get the package source.")?;
		let package_source_artifact = builder
			.get_expression_local(package_source_hash)?
			.into_artifact()
			.context("Expected the package source to be an artifact.")?;
		let mut expression = builder.get_expression_local(package_source_artifact.root)?;
		for component in sub_path.components() {
			expression = builder.get_expression_local(
				expression
					.into_directory()
					.context("Expected a directory.")?
					.entries
					.get(component.as_str())
					.copied()
					.with_context(|| format!(r#"Failed to find file at path {sub_path}"#))?,
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

	async fn load_path_module(&self, path: &Path, sub_path: &Utf8Path) -> Result<String> {
		// Construct the path to the module.
		let mut path = path.to_owned();
		path.push(sub_path);

		// If there is a document for this path, return it.
		if let Some(document) = self.state.open_files.read().await.get(&path) {
			return Ok(document.source.clone());
		}

		// Otherwise, read the file from disk.
		let source = tokio::fs::read_to_string(&path).await?;

		Ok(source)
	}

	async fn load_package_targets(&self, package_hash: Hash) -> Result<String> {
		// Lock the builder.
		let builder = self.state.builder.lock_shared().await?;

		// Produce the package module URL.
		let module_url = js::Url::new_package_module(package_hash, "tangram.ts".into());

		// Get the package's manifest.
		let manifest = builder
			.get_package_manifest(package_hash)
			.await
			.context("Failed to get the package manifest.")?;

		let source = Self::generate_targets_source(&module_url, &manifest, package_hash);

		Ok(source)
	}

	async fn load_path_targets(&self, package_path: &Path) -> Result<String> {
		// Read the manifest.
		let manifest = tokio::fs::read(package_path.join("tangram.json")).await?;
		let manifest = serde_json::from_slice(&manifest)?;

		// Produce the path module URL.
		let module_url = js::Url::new_path_module(package_path.to_owned(), "tangram.ts".into());

		// Generate the source.
		let source = Self::generate_targets_source(&module_url, &manifest, Hash::zero());

		Ok(source)
	}

	/// Generate the code for the targets.
	fn generate_targets_source(
		module_url: &js::Url,
		manifest: &Manifest,
		package_hash: Hash,
	) -> String {
		let mut source = String::new();
		writedoc!(source, r#"import type * as module from "{module_url}";"#,).unwrap();
		source.push('\n');
		writedoc!(
			source,
			r#"let _package: Tangram.Package = await Tangram.getExpression(new Tangram.Hash("{package_hash}"));"#
		)
		.unwrap();
		source.push('\n');
		for target_name in &manifest.targets {
			if target_name == "default" {
				writedoc!(source, r#"export default "#).unwrap();
			} else {
				writedoc!(source, r#"export let {target_name} = "#).unwrap();
			}
			writedoc!(
				source,
				r#"
					(...args: Parameters<typeof module.{target_name}>): Tangram.Target<ReturnType<typeof module.{target_name}>> => new Tangram.Target({{
						package: _package,
						name: "{target_name}",
						args,
					}});
				"#,
			)
			.unwrap();
			source.push('\n');
		}
		source
	}
}
