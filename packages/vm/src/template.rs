use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};

use serde::Deserialize;

use tokio::fs;

/// A bundle of the files required to boot a VM.
#[derive(Debug, Clone, Deserialize)]
pub struct Template {
	pub boot_disk_image: Utf8PathBuf,
	pub kernel: Utf8PathBuf,
	pub kernel_command_line: String,
	pub initrd: Option<Utf8PathBuf>,
	pub guest_agent: Utf8PathBuf,
}

impl Template {
	/// Load a template from a TOML manifest file.
	pub async fn from_manifest(path_to_manifest: &Utf8Path) -> Result<Template> {
		// Get the parent directory. We need this to resolve relative paths.
		let dir = path_to_manifest
			.parent()
			.context("manifest path has no parent directory")?;

		// Load the manifest file
		let manifest_data = fs::read_to_string(path_to_manifest)
			.await
			.context("failed to read VM template manifest")?;

		// Parse the template as TOML.
		let raw: Template =
			toml::from_str(&manifest_data).context("failed to parse manifest TOML")?;

		// Resolve all paths relative to `dir`, and make sure they exist.
		let resolve = |path: Utf8PathBuf, label: &str| -> Result<Utf8PathBuf> {
			let path = dir
				.join(path)
				.canonicalize_utf8()
				.with_context(|| format!("failed to canonicalize path: {label}"))?;
			if let e @ Err(_) = std::fs::metadata(&path) {
				e.with_context(|| format!("file for '{label}' does not exist"))?;
			}
			Ok(path)
		};

		Ok(Template {
			boot_disk_image: resolve(raw.boot_disk_image, "boot_disk_image")?,
			kernel: resolve(raw.kernel, "kernel")?,
			kernel_command_line: raw.kernel_command_line,
			initrd: raw.initrd.map(|p| resolve(p, "initrd")).transpose()?,
			guest_agent: resolve(raw.guest_agent, "guest_agent")?,
		})
	}
}
