use crate::{artifact::Artifact, lockfile::Lockfile, server::Server};
use anyhow::{anyhow, bail, ensure, Result};
use futures::FutureExt;
use std::{path::Path, pin::Pin, sync::Arc};
use url::Url;

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

		let specifier = match specifier.scheme() {
			"fragment" => specifier,

			// Resolve a specifier with scheme "tangram" to a specifier with scheme "fragment" by reading the referrer's lockfile.
			"tangram" => {
				futures::executor::block_on(async {
					// Parse the referrer.
					let referrer = if referrer == "." {
						None
					} else {
						Some(Url::parse(referrer)?)
					};

					let referrer = referrer.ok_or_else(|| {
						anyhow!(r#"A specifier with the scheme "tangram" must have a referrer."#)
					})?;

					// Retrieve the artifact and path from the referrer.
					let domain = referrer
						.domain()
						.ok_or_else(|| anyhow!("URL must have a domain."))?;
					let referrer_artifact: Artifact = domain.parse()?;

					// Create a fragment for the referrer's artifact.
					let fragment = self.server.create_fragment(&referrer_artifact).await?;
					let fragment_path = fragment.path();

					// Read the referrer's lockfile.
					let lockfile_path = fragment_path.join("tangram.lock");
					let lockfile = tokio::fs::read(&lockfile_path).await?;
					let lockfile: Lockfile = serde_json::from_slice(&lockfile)?;

					let specified_package_name = specifier.path();
					let entry = lockfile.0.get(specified_package_name).ok_or_else(|| {
						anyhow!(r#"Could not find package "{specified_package_name}"."#)
					})?;

					let url = format!("fragment://{}/tangram.js", entry.package);
					let url = Url::parse(&url).unwrap();

					Ok::<_, anyhow::Error>(url)
				})?
			},
			_ => {
				bail!(
					r#"The specifier "{specifier}" must have the scheme "fragment" or "tangram"."#,
				)
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
		async move {
			// Ensure the specifier has the scheme "fragment".
			ensure!(
				specifier.scheme() == "fragment",
				r#"The specifier "{specifier}" must have the scheme "fragment"."#,
			);

			// Get the package from the specifier.
			let domain = specifier
				.domain()
				.ok_or_else(|| anyhow!("The specifier must have a domain."))?;
			let specifier_artifact: Artifact = domain.parse()?;

			// Create a fragment for the specifier's artifact.
			let fragment = main_runtime_handle
				.spawn({
					let server = Arc::clone(&server);
					async move { server.create_fragment(&specifier_artifact).await }
				})
				.await
				.unwrap()?;
			let fragment_path = fragment.path();

			// Get the path from the specifier.
			let specifier_path = Path::new(specifier.path());
			let specifier_path = specifier_path.strip_prefix("/").unwrap_or(specifier_path);

			// Read the module's code.
			let module_path = fragment_path.join(specifier_path);
			let code = tokio::fs::read(&module_path).await?;
			let code = String::from_utf8(code)?;

			// Determine the module type.
			let module_type = match specifier_path.extension().and_then(std::ffi::OsStr::to_str) {
				Some("js") => deno_core::ModuleType::JavaScript,
				Some("json") => deno_core::ModuleType::Json,
				Some(extension) => {
					bail!(
						r#"Cannot load module with extension "{extension}" at path "{}"."#,
						module_path.display(),
					);
				},
				None => {
					bail!(
						r#"Cannot load module without extension at path "{}"."#,
						module_path.display(),
					);
				},
			};

			Ok(deno_core::ModuleSource {
				code: code.into_bytes().into_boxed_slice(),
				module_type,
				module_url_specified: specifier.to_string(),
				module_url_found: specifier.to_string(),
			})
		}
		.boxed_local()
	}
}
