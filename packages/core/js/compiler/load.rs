use super::{Compiler, File};
use crate::{hash::Hash, js, manifest::Manifest};
use anyhow::{Context, Result};
use camino::Utf8Path;
use include_dir::include_dir;
use indoc::writedoc;
use std::{fmt::Write, path::Path};
use tokio::io::AsyncReadExt;

static LIB: include_dir::Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/js/compiler/lib");

const LIB_TANGRAM_NS_D_TS: &str = include_str!("../runtime/global.d.ts");

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
			js::Url::TsLib { path } => Self::load_ts_lib(path),
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

		// If there is an opened file for this path, return it.
		if let Some(File::Opened(open_file)) = self.state.files.read().await.get(&path) {
			return Ok(open_file.text.clone());
		}

		// Otherwise, read the file from disk.
		let text = tokio::fs::read_to_string(&path).await?;

		Ok(text)
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

		let text = Self::generate_targets(&module_url, &manifest, package_hash);

		Ok(text)
	}

	async fn load_path_targets(&self, package_path: &Path) -> Result<String> {
		// Read the manifest.
		let manifest = tokio::fs::read(package_path.join("tangram.json")).await?;
		let manifest = serde_json::from_slice(&manifest)?;

		// Produce the path module URL.
		let module_url = js::Url::new_path_module(package_path.to_owned(), "tangram.ts".into());

		// Generate the source.
		let text = Self::generate_targets(&module_url, &manifest, Hash::zero());

		Ok(text)
	}

	/// Generate the code for the targets.
	fn generate_targets(module_url: &js::Url, manifest: &Manifest, package_hash: Hash) -> String {
		let mut code = String::new();
		writedoc!(code, r#"import type * as module from "{module_url}";"#,).unwrap();
		code.push('\n');
		writedoc!(
			code,
			r#"let _package: Tangram.Package = await Tangram.getExpression(new Tangram.Hash("{package_hash}"));"#
		)
		.unwrap();
		code.push('\n');
		for target_name in &manifest.targets {
			if target_name == "default" {
				writedoc!(code, r#"export default "#).unwrap();
			} else {
				writedoc!(code, r#"export let {target_name} = "#).unwrap();
			}
			writedoc!(
				code,
				r#"
					(...args: Parameters<typeof module.{target_name}>): Tangram.Target<Awaited<ReturnType<typeof module.{target_name}>>> => new Tangram.Target({{
						package: _package,
						name: "{target_name}",
						args,
					}});
				"#,
			)
			.unwrap();
			code.push('\n');
		}
		code
	}

	fn load_ts_lib(path: &Utf8Path) -> Result<String> {
		let path = path
			.strip_prefix("/")
			.with_context(|| format!(r#"Path "{path}" is missing a leading slash."#))?;
		let text = match path.as_str() {
			"lib.tangram.ns.d.ts" => LIB_TANGRAM_NS_D_TS,
			_ => LIB
				.get_file(path)
				.with_context(|| format!(r#"Could not find typescript lib for path "{path}"."#))?
				.contents_utf8()
				.context("Failed to read file as UTF-8.")?,
		};
		Ok(text.to_owned())
	}
}
