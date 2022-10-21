use super::Compiler;
use crate::hash::Hash;
use anyhow::{bail, ensure, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use url::Url;

pub const TANGRAM_SCHEME: &str = "tangram";
pub const TANGRAM_MODULE_SCHEME: &str = "tangram-module";
pub const TANGRAM_TARGET_SCHEME: &str = "tangram-target";

impl Compiler {
	pub async fn resolve(&self, specifier: &str, referrer: Option<Url>) -> Result<Url> {
		// Resolve the specifier relative to the referrer.
		let specifier = deno_core::resolve_import(
			specifier,
			referrer.as_ref().map_or(".", |referrer| referrer.as_str()),
		)?;

		let url = match specifier.scheme() {
			// Resolve a specifier with the tangram scheme.
			TANGRAM_SCHEME => self.resolve_tangram(specifier, referrer).await?,

			// Pass through specifiers with the tangram module scheme.
			TANGRAM_MODULE_SCHEME => specifier,

			_ => {
				bail!(r#"The specifier "{specifier}" has an invalid scheme."#,)
			},
		};

		Ok(url)
	}

	async fn resolve_tangram(&self, specifier: Url, referrer: Option<Url>) -> Result<Url> {
		// Ensure there is a referrer.
		let referrer = referrer.with_context(|| {
			format!(r#"A specifier with the scheme "{TANGRAM_SCHEME}" must have a referrer."#)
		})?;

		// Ensure the referrer has the tangram module scheme.
		ensure!(
			referrer.scheme() == TANGRAM_MODULE_SCHEME,
			r#"A specifier with the scheme "{TANGRAM_SCHEME}" must have a referrer whose scheme is "{TANGRAM_MODULE_SCHEME}"."#
		);

		// Retrieve the referrer's package.
		let domain = referrer
			.domain()
			.context("Failed to get domain from the referrer.")?;
		let referrer_package_hash: Hash = domain
			.parse()
			.with_context(|| "Failed to parse referrer domain.")?;

		// Get the referrer's dependencies.
		let referrer_dependencies = self
			.state
			.builder
			.lock_shared()
			.await?
			.get_expression_local(referrer_package_hash)?
			.into_package()
			.context("Expected a package expression.")?
			.dependencies;

		// Get the specifier's package name and sub path.
		let specifier_path = Utf8Path::new(specifier.path());
		let specifier_package_name = specifier_path.components().next().unwrap().as_str();
		let specifier_sub_path = if specifier_path.components().count() > 1 {
			Some(specifier_path.components().skip(1).collect::<Utf8PathBuf>())
		} else {
			None
		};

		// Get the specifier's package hash from the referrer's dependencies.
		let specifier_package_hash = referrer_dependencies.get(specifier_package_name).context(
			"Expected the referrer's package dependencies to contain the specifier's package name.",
		)?;

		// Compute the URL.
		let url = if let Some(specifier_sub_path) = specifier_sub_path {
			format!("{TANGRAM_MODULE_SCHEME}://{specifier_package_hash}/{specifier_sub_path}")
		} else {
			format!("{TANGRAM_TARGET_SCHEME}://{specifier_package_hash}")
		};
		let url = Url::parse(&url).unwrap();

		Ok(url)
	}
}
