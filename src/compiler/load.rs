use super::{Compiler, File};
use crate::{compiler, hash::Hash, manifest::Manifest, util::path_exists};
use anyhow::{bail, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use include_dir::include_dir;
use indoc::{formatdoc, writedoc};
use std::{fmt::Write, path::Path};
use tokio::io::AsyncReadExt;

impl Compiler {
	pub async fn load(&self, url: &compiler::Url) -> Result<String> {
		match url {
			compiler::Url::HashModule(compiler::url::HashModule {
				package_hash,
				module_path,
			}) => self.load_hash_module(*package_hash, module_path).await,

			compiler::Url::HashImport(compiler::url::HashImport { package_hash, .. }) => {
				self.load_hash_import(*package_hash).await
			},

			compiler::Url::HashTarget(compiler::url::HashTarget {
				package_hash,
				module_path,
			}) => Ok(load_hash_target(*package_hash, module_path)),

			compiler::Url::Lib(compiler::url::Lib { path }) => load_lib(path),

			compiler::Url::PathModule(compiler::url::PathModule {
				package_path,
				module_path,
			}) => self.load_path_module(package_path, module_path).await,

			compiler::Url::PathImport(compiler::url::PathImport { package_path, .. }) => {
				self.load_path_import(package_path).await
			},

			compiler::Url::PathTarget(compiler::url::PathTarget {
				package_path,
				module_path,
			}) => Ok(load_path_target(package_path, module_path)),
		}
	}
}

impl Compiler {
	async fn load_hash_module(&self, package_hash: Hash, module_path: &Utf8Path) -> Result<String> {
		// Lock the cli.
		let cli = self.cli.lock_shared().await?;

		// Find the module in the package.
		let package_source_hash = cli
			.get_package_source(package_hash)
			.context("Failed to get the package source.")?;
		let mut expression = cli.get_expression_local(package_source_hash)?;
		for component in module_path.components() {
			expression = cli.get_expression_local(
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
		cli.get_blob(file.blob)
			.await?
			.read_to_string(&mut source)
			.await?;

		Ok(source)
	}

	async fn load_hash_import(&self, target_package_hash: Hash) -> Result<String> {
		// Lock the cli.
		let cli = self.cli.lock_shared().await?;

		// Get the package's entrypoint.
		let entrypoint = cli
			.get_package_entrypoint(target_package_hash)
			.context("Failed to retrieve the package entrypoint.")?
			.context("The package must have a JS entrypoint.")?;

		// Create the URL.
		let url = compiler::Url::new_hash_module(target_package_hash, entrypoint);

		// Get the package's manifest.
		let manifest = cli
			.get_package_manifest(target_package_hash)
			.await
			.context("Failed to get the package manifest.")?;

		let text = generate_import(&url, &manifest);

		Ok(text)
	}
}

fn load_hash_target(package_hash: Hash, module_path: &Utf8Path) -> String {
	generate_target(&compiler::Url::new_hash_module(
		package_hash,
		module_path.to_owned(),
	))
}

const LIB_TANGRAM_D_TS: &str = include_str!("./tangram.d.ts");
const LIB: include_dir::Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/compiler/lib");

#[allow(clippy::module_name_repetitions)]
fn load_lib(path: &Utf8Path) -> Result<String> {
	let path = path
		.strip_prefix("/")
		.with_context(|| format!(r#"Path "{path}" is missing a leading slash."#))?;
	let text = match path.as_str() {
		"lib.tangram.d.ts" => LIB_TANGRAM_D_TS,
		_ => LIB
			.get_file(path)
			.with_context(|| format!(r#"Could not find lib for path "{path}"."#))?
			.contents_utf8()
			.context("Failed to read file as UTF-8.")?,
	};
	Ok(text.to_owned())
}

impl Compiler {
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

	async fn load_path_import(&self, package_path: &Path) -> Result<String> {
		// Get the js entrypoint path.
		let js_entrypoint_path = if path_exists(&package_path.join("tangram.ts")).await? {
			Utf8PathBuf::from("tangram.ts")
		} else if path_exists(&package_path.join("tangram.js")).await? {
			Utf8PathBuf::from("tangram.js")
		} else {
			bail!("No tangram.ts or tangram.js found.");
		};

		// Create the URL.
		let url = compiler::Url::new_path_module(package_path.to_owned(), js_entrypoint_path);

		// Read the package's manifest.
		let manifest = tokio::fs::read(package_path.join("tangram.json")).await?;
		let manifest: Manifest = serde_json::from_slice(&manifest)?;

		// Generate the source.
		let text = generate_import(&url, &manifest);

		Ok(text)
	}
}

fn load_path_target(package_path: &Path, module_path: &Utf8Path) -> String {
	generate_target(&compiler::Url::new_path_module(
		package_path.to_owned(),
		module_path.to_owned(),
	))
}

/// Generate the code for the import.
fn generate_import(
	target_module_url: &compiler::Url,
	target_package_manifest: &Manifest,
) -> String {
	let mut code = String::new();

	// Write the re-export from the target module.
	writedoc!(code, r#"export * from "{target_module_url}";"#).unwrap();
	code.push('\n');

	// If there are no targets, then there is no more code to generate.
	if target_package_manifest.targets.is_empty() {
		return code;
	}

	// Write the core import.
	writedoc!(
		code,
		r#"import {{ getExpression, Hash, Package, Target }} from "tangram:core";"#
	)
	.unwrap();
	code.push('\n');

	// Write the type import from the target module.
	writedoc!(
		code,
		r#"import type * as module from "{target_module_url}";"#
	)
	.unwrap();
	code.push('\n');

	code.push('\n');

	// Get the target package hash.
	let target_package_hash = match target_module_url {
		compiler::Url::HashModule(compiler::url::HashModule { package_hash, .. }) => *package_hash,
		compiler::Url::PathModule(_) => Hash::zero(),
		_ => unreachable!(),
	};

	// Write the export for each target.
	for target_name in &target_package_manifest.targets {
		if target_name == "default" {
			writedoc!(code, r#"export default "#).unwrap();
		} else {
			writedoc!(code, r#"export let {target_name} = "#).unwrap();
		}
		writedoc!(
				code,
				r#"
					(...args: Parameters<typeof module.{target_name}>): Target<Awaited<ReturnType<typeof module.{target_name}>>> => new Target({{
						package: new Hash("{target_package_hash}"),
						name: "{target_name}",
						args,
					}});
				"#,
			)
			.unwrap();
		code.push('\n');
		code.push('\n');
	}

	code
}

/// Generate the code for the process.
fn generate_target(target_module_url: &compiler::Url) -> String {
	formatdoc!(
		r#"
			import {{ addExpression, getExpression, Hash }} from "tangram:core";
			import * as module from "{target_module_url}";

			let targetName = Tangram.syscall("get_name");

			let args = await getExpression(new Hash(Tangram.syscall("get_args")));

			let output = await addExpression(await module[targetName](...args));

			Tangram.syscall("return", output.toString());
		"#
	)
}
