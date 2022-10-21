use crate::js::Compiler;
use anyhow::{bail, Context, Result};
use camino::Utf8Path;
use futures::FutureExt;
use std::{
	collections::HashMap,
	pin::Pin,
	sync::{Arc, Mutex},
};
use url::Url;

pub struct ModuleLoader {
	state: Arc<State>,
}

struct State {
	pub compiler: Compiler,
	pub main_runtime_handle: tokio::runtime::Handle,
	pub modules: Mutex<HashMap<Url, Module, fnv::FnvBuildHasher>>,
}

#[derive(Clone)]
struct Module {
	source: String,
	transpiled_source: Option<String>,
	source_map: Option<String>,
}

impl ModuleLoader {
	/// Create a new module loader.
	pub fn new(compiler: Compiler, main_runtime_handle: tokio::runtime::Handle) -> ModuleLoader {
		let state = State {
			compiler,
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

		// Parse the referrer.
		let referrer = if referrer == "." {
			None
		} else {
			Some(Url::parse(referrer).context("Failed to parse the referrer.")?)
		};

		// Resolve.
		let url = futures::executor::block_on(state.compiler.resolve(specifier, referrer))?;

		Ok(url)
	}

	fn load(
		&self,
		module_specifier: &deno_core::ModuleSpecifier,
		_maybe_referrer: Option<deno_core::ModuleSpecifier>,
		_is_dyn_import: bool,
	) -> Pin<Box<deno_core::ModuleSourceFuture>> {
		let state = Arc::clone(&self.state);
		let specifier = module_specifier.clone();
		self.state
			.main_runtime_handle
			.spawn(async move { load(state, specifier).await })
			.map(std::result::Result::unwrap)
			.boxed_local()
	}
}

async fn load(state: Arc<State>, url: Url) -> Result<deno_core::ModuleSource> {
	// Load the source.
	let source = state.compiler.load(url.clone()).await?;

	// Get the specifier's extension.
	let extension = Utf8Path::new(url.path()).extension();

	// Create the module from the source.
	let module = match extension {
		None | Some("js") => Module {
			source,
			transpiled_source: None,
			source_map: None,
		},

		// If the extension is `.ts` then transpile the source.
		Some("ts") => {
			let transpile_output = state.compiler.transpile(&url, &source)?;
			Module {
				source,
				transpiled_source: Some(transpile_output.transpiled_source),
				source_map: transpile_output.source_map,
			}
		},

		Some(extension) => {
			bail!(r#"Cannot load a module with extension "{extension}"."#);
		},
	};

	// Insert into the modules map.
	state
		.modules
		.lock()
		.unwrap()
		.insert(url.clone(), module.clone());

	// Create the module source.
	let code = module
		.transpiled_source
		.unwrap_or(module.source)
		.into_bytes()
		.into_boxed_slice();
	let module_source = deno_core::ModuleSource {
		code,
		module_type: deno_core::ModuleType::JavaScript,
		module_url_specified: url.to_string(),
		module_url_found: url.to_string(),
	};

	Ok(module_source)
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
		let source_map = module.source_map.as_ref()?;

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
