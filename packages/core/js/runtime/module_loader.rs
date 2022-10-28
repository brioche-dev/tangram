use crate::js::{self, Compiler};
use anyhow::{bail, Context, Result};
use futures::FutureExt;
use std::{
	collections::HashMap,
	pin::Pin,
	sync::{Arc, Mutex},
};

pub struct ModuleLoader {
	state: Arc<State>,
}

struct State {
	pub compiler: Compiler,
	pub main_runtime_handle: tokio::runtime::Handle,
	pub modules: Mutex<HashMap<js::Url, Module, fnv::FnvBuildHasher>>,
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
		// Parse the referrer.
		let referrer = if referrer == "." {
			None
		} else {
			Some(referrer.parse().context("Failed to parse the referrer.")?)
		};

		// Block this thread using a synchronous channel while resolution runs on the main runtime.
		let (sender, receiver) = std::sync::mpsc::channel();
		self.state.main_runtime_handle.spawn({
			let state = Arc::clone(&self.state);
			let specifier = specifier.to_owned();
			let referrer = referrer.clone();
			async move {
				let result = state.compiler.resolve(&specifier, referrer.as_ref()).await;
				sender.send(result).unwrap();
			}
		});
		let url = receiver.recv().unwrap().with_context(|| {
			format!(
				r#"Failed to resolve specifier "{specifier}" relative to referrer "{referrer:?}"."#
			)
		})?;

		Ok(url.into())
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
			.spawn(async move {
				let specifier = specifier.try_into()?;
				let module_source = load(state, &specifier).await?;
				Ok(module_source)
			})
			.map(std::result::Result::unwrap)
			.boxed_local()
	}
}

async fn load(state: Arc<State>, url: &js::Url) -> Result<deno_core::ModuleSource> {
	// Load the source.
	let source = state.compiler.load(url).await?;

	// Determine if the module should be transpiled.
	let transpile = match url {
		js::Url::PackageModule { module_path, .. } => {
			// Get the module's path extension.
			let extension = module_path
				.extension()
				.with_context(|| format!(r#"Cannot load from URL "{url}" with no extension."#))?;
			match extension {
				"js" => false,
				"ts" => true,
				_ => {
					bail!(r#"Cannot load from URL with extension "{extension}"."#);
				},
			}
		},
		js::Url::PackageTargets { .. } => true,
		_ => {
			bail!(r#"Cannot load from URL "{url}"."#);
		},
	};

	// Transpile the module if necessary.
	let module = if transpile {
		let transpile_output = state.compiler.transpile(url, &source)?;
		Module {
			source,
			transpiled_source: Some(transpile_output.transpiled_source),
			source_map: transpile_output.source_map,
		}
	} else {
		Module {
			source,
			transpiled_source: None,
			source_map: None,
		}
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

		// Parse the file name as a URL.
		let specifier = file_name.parse().ok()?;

		// Retrieve the module.
		let module = modules.get(&specifier)?;

		// Retrieve the source map.
		let source_map = module.source_map.as_ref()?;

		Some(source_map.clone().into_bytes())
	}

	fn get_source_line(&self, file_name: &str, line_number: usize) -> Option<String> {
		// Lock the modules.
		let modules = self.state.modules.lock().unwrap();

		// Parse the file name as a URL.
		let specifier = file_name.parse().ok()?;

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
