use crate::{builder, hash::Hash, util::path_exists};
use anyhow::{anyhow, bail, ensure, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use futures::FutureExt;
use indoc::writedoc;
use std::{
	collections::HashMap,
	fmt::Write,
	pin::Pin,
	sync::{Arc, Mutex},
};
use url::Url;

pub const TANGRAM_SCHEME: &str = "tangram";
pub const TANGRAM_MODULE_SCHEME: &str = "tangram-module";
pub const TANGRAM_TARGET_PROXY_SCHEME: &str = "tangram-target-proxy";

pub struct ModuleLoader {
	state: Arc<State>,
}

struct State {
	pub builder: builder::Shared,
	pub main_runtime_handle: tokio::runtime::Handle,
	pub modules: Mutex<HashMap<Url, Module, fnv::FnvBuildHasher>>,
}

struct Module {
	source: String,
	transpiled_source: deno_ast::TranspiledSource,
}

impl ModuleLoader {
	/// Create a new module loader.
	pub fn new(
		builder: builder::Shared,
		main_runtime_handle: tokio::runtime::Handle,
	) -> ModuleLoader {
		let state = State {
			builder,
			main_runtime_handle,
			modules: Mutex::new(HashMap::default()),
		};
		ModuleLoader {
			state: Arc::new(state),
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
		let state = Arc::clone(&self.state);

		// Resolve the specifier relative to the referrer.
		let specifier = deno_core::resolve_import(specifier, referrer)?;

		// Parse the referrer.
		let referrer = if referrer == "." {
			None
		} else {
			Some(Url::parse(referrer)?)
		};

		let specifier = match specifier.scheme() {
			// Resolve a specifier with the tangram scheme.
			TANGRAM_SCHEME => {
				futures::executor::block_on(resolve_tangram(&state, specifier, referrer))?
			},

			// Resolve a specifier with the tangram module scheme.
			TANGRAM_MODULE_SCHEME => {
				futures::executor::block_on(resolve_tangram_module(&state, specifier, referrer))?
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
		maybe_referrer: Option<deno_core::ModuleSpecifier>,
		_is_dyn_import: bool,
	) -> Pin<Box<deno_core::ModuleSourceFuture>> {
		let state = Arc::clone(&self.state);
		let referrer = maybe_referrer;
		let specifier = module_specifier.clone();
		self.state
			.main_runtime_handle
			.spawn(async move {
				match specifier.scheme() {
					// Load a module with the tangram module scheme.
					TANGRAM_MODULE_SCHEME => load_tangram_module(&state, specifier, referrer).await,

					// Load a module with the tangram target proxy scheme.
					TANGRAM_TARGET_PROXY_SCHEME => {
						load_tangram_target_proxy(&state, specifier, referrer).await
					},

					_ => {
						bail!(r#"The specifier has an unsupported scheme."#);
					},
				}
			})
			.map(std::result::Result::unwrap)
			.boxed_local()
	}
}

#[allow(clippy::unused_async)]
async fn resolve_tangram(
	state: &State,
	specifier: deno_core::ModuleSpecifier,
	referrer: Option<deno_core::ModuleSpecifier>,
) -> Result<deno_core::ModuleSpecifier> {
	// Ensure there is a referrer.
	let referrer = referrer.ok_or_else(|| {
		anyhow!(r#"A specifier with the scheme "{TANGRAM_SCHEME}" must have a referrer."#)
	})?;

	// Ensure the referrer has the tangram module scheme.
	ensure!(
		referrer.scheme() == TANGRAM_MODULE_SCHEME,
		r#"A specifier with the scheme "{TANGRAM_SCHEME}" must have a referrer whose scheme is "{TANGRAM_MODULE_SCHEME}"."#
	);

	// Retrieve the referrer's package.
	let domain = referrer
		.domain()
		.ok_or_else(|| anyhow!("Failed to get domain from the referrer."))?;
	let referrer_package_hash: Hash = domain
		.parse()
		.with_context(|| "Failed to parse referrer domain.")?;

	// Get the specifier's package name and sub path.
	let specifier_path = Utf8Path::new(specifier.path());
	let specifier_package_name = specifier_path.components().next().unwrap().as_str();
	let specifier_sub_path = if specifier_path.components().count() > 1 {
		Some(specifier_path.components().skip(1).collect::<Utf8PathBuf>())
	} else {
		None
	};

	// Get the referrer's dependencies.
	let referrer_dependencies = state
		.builder
		.get_expression(referrer_package_hash)
		.await?
		.into_package()
		.ok_or_else(|| anyhow!("Expected a package expression."))?
		.dependencies;

	// Look up the specifier's package name in the referrer's dependencies.
	let specifier_package_hash = referrer_dependencies
		.get(specifier_package_name)
		.ok_or_else(|| {
			anyhow!(
				r#"Expected the referrer's package dependencies to contain the specifier's package name."#
			)
		})?;

	// Compute the URL to resolve to.
	let url = if let Some(specifier_sub_path) = specifier_sub_path {
		format!("{TANGRAM_MODULE_SCHEME}://{specifier_package_hash}/{specifier_sub_path}")
	} else {
		format!("{TANGRAM_TARGET_PROXY_SCHEME}://{specifier_package_hash}")
	};
	let url = Url::parse(&url).unwrap();

	Ok(url)
}

#[allow(clippy::unused_async)]
async fn resolve_tangram_module(
	_state: &State,
	specifier: deno_core::ModuleSpecifier,
	_referrer: Option<deno_core::ModuleSpecifier>,
) -> Result<deno_core::ModuleSpecifier> {
	Ok(specifier)
}

async fn load_tangram_module(
	state: &State,
	specifier: deno_core::ModuleSpecifier,
	_referrer: Option<deno_core::ModuleSpecifier>,
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
	let specifier_package_hash: Hash = domain.parse()?;

	// Checkout specifier's package.
	let specifier_package_source_hash = state
		.builder
		.get_package_source(specifier_package_hash)
		.await?;
	let artifact_path = state
		.builder
		.checkout_to_artifacts(specifier_package_source_hash)
		.await?;

	// Get the path from the specifier.
	let specifier_path = Utf8Path::new(specifier.path());
	let specifier_path = specifier_path
		.strip_prefix("/")
		.with_context(|| "The specifier must have a leading slash.")?;

	// Get the module path.
	let module_path = artifact_path.join(specifier_path);
	let (module_path, is_typescript) = if path_exists(&module_path).await? {
		(module_path, false)
	} else if path_exists(&module_path.with_extension("ts")).await? {
		(module_path.with_extension("ts"), true)
	} else {
		bail!(
			r#"Failed to find a module at path "{}"."#,
			module_path.display(),
		);
	};

	// Read the module's source.
	let source = tokio::fs::read(&module_path).await.with_context(|| {
		format!(
			r#"Failed to read file at path "{}"."#,
			module_path.display(),
		)
	})?;
	let source = String::from_utf8(source)?;

	// Transpile the code if necessary.
	let code = if is_typescript {
		// Parse the code.
		let parsed_source = deno_ast::parse_module(deno_ast::ParseParams {
			specifier: specifier.to_string(),
			text_info: deno_ast::SourceTextInfo::new(source.clone().into()),
			media_type: deno_ast::MediaType::TypeScript,
			capture_tokens: true,
			scope_analysis: true,
			maybe_syntax: None,
		})
		.with_context(|| format!(r#"Failed to parse the module with URL "{specifier}"."#))?;

		// Transpile the code.
		let transpiled_source = parsed_source
			.transpile(&deno_ast::EmitOptions {
				inline_source_map: false,
				..Default::default()
			})
			.with_context(|| {
				format!(r#"Failed to transpile the module with URL "{specifier}"."#)
			})?;
		let code = transpiled_source.text.clone();

		// Insert into the modules map.
		let module = Module {
			source,
			transpiled_source,
		};
		state
			.modules
			.lock()
			.unwrap()
			.insert(specifier.clone(), module);

		code
	} else {
		source
	};

	// Determine the module type.
	let module_type = match specifier_path.extension() {
		Some("js") | None => deno_core::ModuleType::JavaScript,
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

async fn load_tangram_target_proxy(
	state: &State,
	specifier: deno_core::ModuleSpecifier,
	_referrer: Option<deno_core::ModuleSpecifier>,
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
	let package_hash: Hash = domain
		.parse()
		.context("Failed to parse the domain as a hash.")?;

	// Get the package's manifest.
	let manifest = state
		.builder
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

	Ok(deno_core::ModuleSource {
		code: code.into_bytes().into_boxed_slice(),
		module_type: deno_core::ModuleType::JavaScript,
		module_url_specified: specifier.to_string(),
		module_url_found: specifier.to_string(),
	})
}

impl deno_core::SourceMapGetter for ModuleLoader {
	fn get_source_map(&self, file_name: &str) -> Option<Vec<u8>> {
		// Lock the modules.
		let modules = self.state.modules.lock().unwrap();

		// Parse the file name.
		let specifier = Url::parse(file_name).ok()?;

		// Retrieve the module.
		let module = modules.get(&specifier)?;

		// Retrieve the source map.
		let source_map = module.transpiled_source.source_map.as_ref()?;

		Some(source_map.clone().into_bytes())
	}

	fn get_source_line(&self, file_name: &str, line_number: usize) -> Option<String> {
		// Lock the modules.
		let modules = self.state.modules.lock().unwrap();

		// Parse the file name.
		let specifier = Url::parse(file_name).ok()?;

		// Retrieve the transpiled source.
		let module = modules.get(&specifier)?;

		// Retrieve the line.
		module
			.source
			.split('\n')
			.nth(line_number)
			.map(ToOwned::to_owned)
	}
}
