use crate::{artifact::Artifact, lockfile::Lockfile, manifest::Manifest, server::Server};
use anyhow::{anyhow, bail, ensure, Result};
use camino::{Utf8Path, Utf8PathBuf};
use futures::FutureExt;
use indoc::formatdoc;
use std::{pin::Pin, sync::Arc};
use url::Url;

pub const TANGRAM_SCHEME: &str = "tangram";
pub const TANGRAM_MODULE_SCHEME: &str = "tangram-module";
pub const TANGRAM_TARGET_PROXY_SCHEME: &str = "tangram-target-proxy";

pub struct ModuleLoader {
	server: Arc<Server>,
	main_runtime_handle: tokio::runtime::Handle,
}

impl ModuleLoader {
	pub fn new(server: Arc<Server>, main_runtime_handle: tokio::runtime::Handle) -> ModuleLoader {
		ModuleLoader {
			server,
			main_runtime_handle,
		}
	}
}

impl deno_core::ModuleLoader for ModuleLoader {
	fn resolve(
		&self,
		specifier: &str,
		referrer: &str,
		_is_main: bool,
	) -> Result<deno_core::ModuleSpecifier> {
		// Resolve the specifier relative to the referrer.
		let specifier = deno_core::resolve_import(specifier, referrer)?;

		// Parse the referrer.
		let referrer = if referrer == "." {
			None
		} else {
			Some(Url::parse(referrer)?)
		};

		let specifier = match specifier.scheme() {
			// Resolve a specifier with tangram scheme.
			TANGRAM_SCHEME => {
				futures::executor::block_on(self.resolve_tangram(specifier, referrer))?
			},

			// Resolve a specifier with the tangram scheme.
			TANGRAM_MODULE_SCHEME => {
				futures::executor::block_on(self.resolve_tangram_module(specifier, referrer))?
			},

			_ => {
				bail!(r#"The specifier "{specifier}" has an invalid scheme."#,)
			},
		};

		Ok(specifier)
	}

	fn load(
		&self,
		module_specifier: &deno_core::ModuleSpecifier,
		_maybe_referrer: Option<deno_core::ModuleSpecifier>,
		_is_dyn_import: bool,
	) -> Pin<Box<deno_core::ModuleSourceFuture>> {
		let server = Arc::clone(&self.server);
		let main_runtime_handle = self.main_runtime_handle.clone();
		let specifier = module_specifier.clone();
		main_runtime_handle
			.spawn(async move {
				match specifier.scheme() {
					// Load a module with the tangram module scheme.
					TANGRAM_MODULE_SCHEME => {
						ModuleLoader::load_tangram_module(server, specifier).await
					},

					// Load a module with the tangram target proxy scheme.
					TANGRAM_TARGET_PROXY_SCHEME => {
						ModuleLoader::load_tangram_target_proxy(server, specifier).await
					},

					_ => {
						bail!(r#"The specifier has an unsupported scheme."#);
					},
				}
			})
			.map(|spawn_result| spawn_result.unwrap())
			.boxed_local()
	}
}

impl ModuleLoader {
	async fn resolve_tangram(
		&self,
		specifier: deno_core::ModuleSpecifier,
		referrer: Option<deno_core::ModuleSpecifier>,
	) -> Result<deno_core::ModuleSpecifier> {
		let referrer = referrer.ok_or_else(|| {
			anyhow!(r#"A specifier with the scheme "{TANGRAM_SCHEME}" must have a referrer."#)
		})?;

		ensure!(
			referrer.scheme() == TANGRAM_MODULE_SCHEME,
			r#"A specifier with the scheme "{TANGRAM_SCHEME}" must have a referrer whose scheme is "{TANGRAM_MODULE_SCHEME}"."#
		);

		// Retrieve the package and path from the referrer.
		let domain = referrer
			.domain()
			.ok_or_else(|| anyhow!("URL must have a domain."))?;
		let referrer_package: Artifact = domain.parse()?;

		// Create a fragment for the referrer's package.
		let referrer_fragment = self.server.create_fragment(&referrer_package).await?;
		let referrer_fragment_path = referrer_fragment.path();

		// Read the referrer's lockfile.
		let referrer_lockfile_path = referrer_fragment_path.join("tangram.lock");
		let referrer_lockfile = tokio::fs::read(&referrer_lockfile_path).await?;
		let referrer_lockfile: Lockfile = serde_json::from_slice(&referrer_lockfile)?;

		let specifier_path = Utf8Path::new(specifier.path());
		let specifier_package_name = specifier_path.components().next().unwrap().as_str();
		let specifier_sub_path = if specifier_path.components().count() > 1 {
			Some(specifier_path.components().skip(1).collect::<Utf8PathBuf>())
		} else {
			None
		};
		let lockfile_entry = referrer_lockfile
			.0
			.get(specifier_package_name)
			.ok_or_else(|| anyhow!(r#"Could not find package "{specifier_package_name}"."#))?;
		let specifier_package = lockfile_entry.package;

		let url = if let Some(specifier_sub_path) = specifier_sub_path {
			format!("{TANGRAM_MODULE_SCHEME}://{specifier_package}/{specifier_sub_path}",)
		} else {
			format!("{TANGRAM_TARGET_PROXY_SCHEME}://{specifier_package}")
		};
		let url = Url::parse(&url).unwrap();

		Ok(url)
	}

	async fn resolve_tangram_module(
		&self,
		specifier: deno_core::ModuleSpecifier,
		_referrer: Option<deno_core::ModuleSpecifier>,
	) -> Result<deno_core::ModuleSpecifier> {
		Ok(specifier)
	}

	async fn load_tangram_target_proxy(
		server: Arc<Server>,
		specifier: deno_core::ModuleSpecifier,
	) -> Result<deno_core::ModuleSource> {
		// Ensure the specifier has the tangram target proxy scheme.
		ensure!(
			specifier.scheme() == TANGRAM_TARGET_PROXY_SCHEME,
			r#"The specifier "{specifier}" must have the scheme "{TANGRAM_TARGET_PROXY_SCHEME}"."#,
		);

		// Get the package from the specifier.
		let domain = specifier
			.domain()
			.ok_or_else(|| anyhow!("The specifier must have a domain."))?;
		let specifier_artifact: Artifact = domain.parse()?;
		let specifier_artifact_json = serde_json::to_string(&specifier_artifact)?;

		// Create a fragment for the specifier's package.
		let fragment = server.create_fragment(&specifier_artifact).await?;
		let fragment_path = fragment.path();

		let manifest = tokio::fs::read(&fragment_path.join("tangram.json")).await?;
		let manifest: Manifest = serde_json::from_slice(&manifest)?;

		let mut code = String::new();

		for target_name in manifest.targets {
			code.push_str(&formatdoc!(
				r#"
					export let {target_name} = (args) => {{
						return Tangram.target({{
							lockfile: {{}},
							package: {specifier_artifact_json},
							name: "{target_name}",
							args,
						}});
					}}
				"#,
			));
		}

		Ok(deno_core::ModuleSource {
			code: code.into_bytes().into_boxed_slice(),
			module_type: deno_core::ModuleType::JavaScript,
			module_url_specified: specifier.to_string(),
			module_url_found: specifier.to_string(),
		})
	}

	async fn load_tangram_module(
		server: Arc<Server>,
		specifier: deno_core::ModuleSpecifier,
	) -> Result<deno_core::ModuleSource> {
		// Ensure the specifier has the tangram module scheme.
		ensure!(
			specifier.scheme() == TANGRAM_MODULE_SCHEME,
			r#"The specifier "{specifier}" must have the scheme "{TANGRAM_MODULE_SCHEME}"."#,
		);

		// Get the package from the specifier.
		let domain = specifier
			.domain()
			.ok_or_else(|| anyhow!("The specifier must have a domain."))?;
		let specifier_artifact: Artifact = domain.parse()?;

		// Create a fragment for the specifier's package.
		let fragment = server.create_fragment(&specifier_artifact).await?;
		let fragment_path = fragment.path();

		// Get the path from the specifier.
		let specifier_path = Utf8Path::new(specifier.path());
		let specifier_path = specifier_path.strip_prefix("/").unwrap_or(specifier_path);

		// Read the module's code.
		let module_path = fragment_path.join(specifier_path);
		let code = tokio::fs::read(&module_path).await?;
		let code = String::from_utf8(code)?;

		// Determine the module type.
		let module_type = match specifier_path.extension() {
			Some("js") => deno_core::ModuleType::JavaScript,
			Some("json") => deno_core::ModuleType::Json,
			_ => {
				bail!(r#"Cannot load module at path "{}"."#, module_path.display());
			},
		};

		Ok(deno_core::ModuleSource {
			code: code.into_bytes().into_boxed_slice(),
			module_type,
			module_url_specified: specifier.to_string(),
			module_url_found: specifier.to_string(),
		})
	}
}
