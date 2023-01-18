use super::{ModuleIdentifier, TrackedFile};
use crate::{package::PackageHash, Cli};
use anyhow::{Context, Result};
use camino::Utf8Path;
use include_dir::include_dir;
use std::path::Path;
use tokio::io::AsyncReadExt;

impl Cli {
	/// Load a module with the given module identifier.
	pub async fn load(&self, module_identifier: &ModuleIdentifier) -> Result<String> {
		match module_identifier {
			ModuleIdentifier::Lib { path } => load_lib(path),

			ModuleIdentifier::Core { path } => load_core(path),

			ModuleIdentifier::Hash {
				package_hash,
				module_path,
			} => self.load_hash(*package_hash, module_path).await,

			ModuleIdentifier::Path {
				package_path,
				module_path,
			} => self.load_path(package_path, module_path).await,
		}
	}
}

const LIB_TANGRAM_D_TS: &str = include_str!("./tangram.d.ts");
const LIB: include_dir::Dir = include_dir!("$CARGO_MANIFEST_DIR/src/compiler/lib");

fn load_lib(path: &Utf8Path) -> Result<String> {
	let path = path
		.strip_prefix("/")
		.with_context(|| format!(r#"The path "{path}" is missing a leading slash."#))?;
	let text = match path.as_str() {
		"lib.tangram.d.ts" => LIB_TANGRAM_D_TS,
		_ => LIB
			.get_file(path)
			.with_context(|| format!(r#"Could not find a lib with the path "{path}"."#))?
			.contents_utf8()
			.context("Failed to read the file as UTF-8.")?,
	};
	Ok(text.to_owned())
}

const CORE: include_dir::Dir = include_dir!("$CARGO_MANIFEST_DIR/src/compiler/core");

fn load_core(path: &Utf8Path) -> Result<String> {
	let path = path
		.strip_prefix("/")
		.with_context(|| format!(r#"Path "{path}" is missing a leading slash."#))?;
	let text = CORE
		.get_file(path)
		.with_context(|| format!(r#"Could not find core path "{path}"."#))?
		.contents_utf8()
		.context("Failed to read the file as UTF-8.")?;
	Ok(text.to_owned())
}

impl Cli {
	async fn load_hash(&self, package_hash: PackageHash, module_path: &Utf8Path) -> Result<String> {
		// Find the module in the package.
		let package_source_artifact_hash = self
			.get_package_source(package_hash)
			.context("Failed to get the package source.")?;
		let mut artifact = self.get_artifact_local(package_source_artifact_hash)?;
		for component in module_path.components() {
			artifact = self.get_artifact_local(
				artifact
					.into_directory()
					.context("Expected a directory.")?
					.entries
					.get(component.as_str())
					.copied()
					.with_context(|| format!(r#"Failed to find file at path {module_path}"#))?,
			)?;
		}

		// Read the module.
		let file = artifact.into_file().context("Expected a file.")?;
		let mut source = String::new();
		self.get_blob(file.blob)
			.await?
			.read_to_string(&mut source)
			.await?;

		Ok(source)
	}
}

impl Cli {
	async fn load_path(&self, package_path: &Path, module_path: &Utf8Path) -> Result<String> {
		anyhow::ensure!(
			module_path.extension() == Some("tg"),
			"Tried loading file {module_path}, but can only load .tg files."
		);

		// Construct the path to the module.
		let path = package_path.join(module_path);

		// If there is a tracked file for this path that is open, return its content.
		if let Some(TrackedFile::Opened(opened_tracked_file)) =
			self.inner.tracked_files.read().await.get(&path)
		{
			return Ok(opened_tracked_file.text.clone());
		}

		// Otherwise, read the file from disk.
		let text = tokio::fs::read_to_string(&path).await?;

		Ok(text)
	}
}
