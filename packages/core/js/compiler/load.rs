use super::{Compiler, File};
use crate::{hash::Hash, js, manifest::Manifest, util::path_exists};
use anyhow::{bail, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use include_dir::include_dir;
use indoc::writedoc;
use std::{fmt::Write, path::Path};
use tokio::io::AsyncReadExt;

const BUILTINS: &str = include_str!("../runtime/builtins.ts");
const LIB_TANGRAM_D_TS: &str = include_str!("../runtime/global.d.ts");
const LIB: include_dir::Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/js/compiler/lib");

impl Compiler {
	pub async fn load(&self, url: &js::Url) -> Result<String> {
		match url {
			js::Url::Builtins { path } => load_builtins(path),
			js::Url::Lib { path } => load_lib(path),
			js::Url::PackageModule {
				package_hash,
				module_path,
			} => self.load_package_module(*package_hash, module_path).await,
			js::Url::PackageTargets { package_hash } => {
				self.load_package_targets(*package_hash).await
			},
			js::Url::PathModule {
				package_path,
				module_path,
			} => self.load_path_module(package_path, module_path).await,
			js::Url::PathTargets { package_path } => self.load_path_targets(package_path).await,
		}
	}
}

#[allow(clippy::module_name_repetitions)]
fn load_builtins(path: &Utf8Path) -> Result<String> {
	let path = path
		.strip_prefix("/")
		.with_context(|| format!(r#"Path "{path}" is missing a leading slash."#))?;
	let text = match path.as_str() {
		"lib.ts" => BUILTINS,
		_ => bail!(r#"Unable to find builtins with path "{path}"."#),
	};
	Ok(text.to_owned())
}

#[allow(clippy::module_name_repetitions)]
fn load_lib(path: &Utf8Path) -> Result<String> {
	let path = path
		.strip_prefix("/")
		.with_context(|| format!(r#"Path "{path}" is missing a leading slash."#))?;
	let text = match path.as_str() {
		"lib.tangram.d.ts" => LIB_TANGRAM_D_TS,
		_ => LIB
			.get_file(path)
			.with_context(|| format!(r#"Could not find typescript lib for path "{path}"."#))?
			.contents_utf8()
			.context("Failed to read file as UTF-8.")?,
	};
	Ok(text.to_owned())
}

impl Compiler {
	async fn load_package_module(
		&self,
		package_hash: Hash,
		module_path: &Utf8Path,
	) -> Result<String> {
		// Lock the builder.
		let builder = self.state.builder.lock_shared().await?;

		// Find the module in the package.
		let package_source_hash = builder
			.get_package_source(package_hash)
			.context("Failed to get the package source.")?;
		let mut expression = builder.get_expression_local(package_source_hash)?;
		for component in module_path.components() {
			expression = builder.get_expression_local(
				expression
					.into_directory()
					.context("Expected a directory.")?
					.entries
					.get(component.as_str())
					.copied()
					.with_context(|| format!(r#"Failed to find file at path {module_path}"#))?,
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

	async fn load_package_targets(&self, package_hash: Hash) -> Result<String> {
		// Lock the builder.
		let builder = self.state.builder.lock_shared().await?;

		// Get the package's JS entrypoint path.
		let js_entrypoint_path = self
			.state
			.builder
			.lock_shared()
			.await?
			.get_package_js_entrypoint(package_hash)
			.context("Failed to retrieve the package JS entrypoint.")?
			.context("The package must have a JS entrypoint.")?;

		// Produce the package module URL.
		let module_url = js::Url::new_package_module(package_hash, js_entrypoint_path);

		// Get the package's manifest.
		let manifest = builder
			.get_package_manifest(package_hash)
			.await
			.context("Failed to get the package manifest.")?;

		let text = generate_targets(&module_url, &manifest, package_hash);

		Ok(text)
	}

	async fn load_path_module(
		&self,
		package_path: &Path,
		module_path: &Utf8Path,
	) -> Result<String> {
		// Construct the path to the module.
		let path = package_path.join(module_path);

		// If there is an opened file for this path, return it.
		if let Some(File::Opened(open_file)) = self.state.files.read().await.get(&path) {
			return Ok(open_file.text.clone());
		}

		// Otherwise, read the file from disk.
		let text = tokio::fs::read_to_string(&path).await?;

		Ok(text)
	}

	async fn load_path_targets(&self, package_path: &Path) -> Result<String> {
		// Read the manifest.
		let manifest = tokio::fs::read(package_path.join("tangram.json")).await?;
		let manifest = serde_json::from_slice(&manifest)?;

		// Get the js entrypoint path.
		let js_entrypoint_path = if path_exists(&package_path.join("tangram.ts")).await? {
			Utf8PathBuf::from("tangram.ts")
		} else if path_exists(&package_path.join("tangram.js")).await? {
			Utf8PathBuf::from("tangram.js")
		} else {
			bail!("No tangram.ts or tangram.js found.");
		};

		// Produce the path module URL.
		let module_url = js::Url::new_path_module(package_path.to_owned(), js_entrypoint_path);

		// Generate the source.
		let text = generate_targets(&module_url, &manifest, Hash::zero());

		Ok(text)
	}
}

/// Generate the code for the targets.
fn generate_targets(module_url: &js::Url, manifest: &Manifest, package_hash: Hash) -> String {
	let mut code = String::new();
	writedoc!(
		code,
		r#"import {{ getExpression, Hash, Package, Target }} from "tangram-builtins:///lib.ts";"#
	)
	.unwrap();
	writedoc!(code, r#"import type * as module from "{module_url}";"#).unwrap();
	code.push('\n');
	writedoc!(
		code,
		r#"let _package: Package = await getExpression(new Hash("{package_hash}"));"#
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
					(...args: Parameters<typeof module.{target_name}>): Target<Awaited<ReturnType<typeof module.{target_name}>>> => new Target({{
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
