use crate::{builder::Builder, expression, hash::Hash, lockfile::Lockfile};
use anyhow::{anyhow, bail, Context, Result};
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use deno_core::{serde_v8, v8};
use futures::Future;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use std::{env, fs, io};
use tokio::sync::oneshot;
use tracing::{debug, error, trace};

// TODO: Compress this snapshot with zstd to save 20MB of binary size (and presumably some startup
// time too)
const TS_SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/js_compiler_snapshot"));

/// Concatenate together all the `.d.ts` files which define the runtime environment.
const ENVIRONMENT_DEFINITIONS: &str = concat!(
	include_str!("ts_defs/hacks.d.ts"),
	include_str!("ts_defs/tangram_console.d.ts"),
	include_str!("../runtime/global.d.ts"),
	include_str!("ts_defs/lib.d.ts"),
);

pub struct Runtime {
	runtime: deno_core::JsRuntime,
	_state: Arc<OpState>,
}

struct OpState {
	builder: Builder,
	main_runtime_handle: tokio::runtime::Handle,
}

impl Runtime {
	/// Create a new Compiler (`typescript` inside a `deno_core::JsRuntime`).
	#[must_use]
	pub fn new(builder: Builder) -> Runtime {
		let main_runtime_handle = tokio::runtime::Handle::current();

		let state = Arc::new(OpState {
			builder,
			main_runtime_handle,
		});

		// Build the tangram extension.
		let tangram_extension = deno_core::Extension::builder()
			.ops(vec![
				op_tg_console_log::decl(),
				op_tg_console_error::decl(),
				op_tg_read_file::decl(),
				op_tg_file_exists::decl(),
				op_tg_resolve::decl(),
			])
			.state({
				{
					let state: Arc<OpState> = Arc::clone(&state);
					move |state_map| {
						state_map.put(Arc::clone(&state));
						Ok(())
					}
				}
			})
			.build();

		// Create the js runtime.
		let runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
			extensions: vec![tangram_extension],
			module_loader: None,
			startup_snapshot: Some(deno_core::Snapshot::Static(TS_SNAPSHOT)),
			..Default::default()
		});

		Runtime {
			_state: state,
			runtime,
		}
	}

	pub async fn handle(&mut self, request: Request) -> Result<Response> {
		// Create a scope to call the handle function.
		let mut scope = self.runtime.handle_scope();
		let mut try_catch_scope = v8::TryCatch::new(&mut scope);

		// Get the handle function.
		let handle: v8::Local<v8::Function> =
			deno_core::JsRuntime::grab_global(&mut try_catch_scope, "handle")
				.context("Failed to get the handle function from the global scope.")?;

		// Call the handle function.
		let receiver = v8::undefined(&mut try_catch_scope).into();
		let request = serde_v8::to_v8(&mut try_catch_scope, request)
			.context("Failed to serialize the request.")?;
		let output = handle.call(&mut try_catch_scope, receiver, &[request]);

		// Handle an exception from js.
		if try_catch_scope.has_caught() {
			let exception = try_catch_scope.exception().unwrap();
			let mut scope = v8::HandleScope::new(&mut try_catch_scope);
			let error = deno_core::error::JsError::from_v8_exception(&mut scope, exception);
			return Err(error.into());
		}

		// If there was no caught exception then retrieve the return value.
		let output = output.unwrap();

		// Move the return value to the global scope.
		let output = v8::Global::new(&mut try_catch_scope, output);
		drop(try_catch_scope);
		drop(scope);

		// Resolve the value.
		let output = self.runtime.resolve_value(output).await?;

		// Deserialize the response.
		let mut scope = self.runtime.handle_scope();
		let output = v8::Local::new(&mut scope, output);
		let response =
			serde_v8::from_v8(&mut scope, output).context("Failed to deserialize the response.")?;
		drop(scope);

		Ok(response)
	}
}

#[derive(serde::Serialize)]
#[serde(tag = "type", content = "request", rename_all = "camelCase")]
pub enum Request {
	Check(CheckRequest),
}

#[derive(serde::Deserialize)]
#[serde(tag = "type", content = "response", rename_all = "camelCase")]
pub enum Response {
	Check(CheckResponse),
}

pub struct Envelope {
	pub request: Request,
	pub sender: oneshot::Sender<Result<Response>>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckRequest {
	pub file_names: Vec<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckResponse {
	pub diagnostics: Vec<super::Diagnostic>,
}

/// Returns whether or not this path is a "special" path in the VFS (e.g. whether it begins with
/// `/__tangram__`).
fn is_special_path(path: &Utf8Path) -> bool {
	let first_two_components = path.components().take(2).collect::<Vec<_>>();
	first_two_components == vec![Utf8Component::RootDir, Utf8Component::Normal("__tangram__")]
}

/// Check out a package with the given hash to disk, and return the path to the checkout.
async fn check_out_package(builder: &Builder, hash: Hash) -> Result<Utf8PathBuf> {
	// TODO: Delete this function. Read directly from the store to avoid having to check things out
	let builder = builder.lock_shared().await?;

	let source_hash = builder
		.get_package_source(hash)
		.context("Failed to get package source")?;
	let artifact_path = builder
		.checkout_to_artifacts(source_hash)
		.await
		.context("Failed to check out package source")?;
	let artifact_path_utf8 =
		Utf8PathBuf::try_from(artifact_path).context("Path to checkout was not UTF-8")?;
	Ok(artifact_path_utf8)
}

/// Read a file from the virtual filesystem exposed to the JS runtime.
///
/// If the path starts with `/__tangram__/`, this will resolve files internally or from the store.
async fn vfs_read_file(builder: &Builder, path: &str) -> Result<Option<Box<dyn io::Read>>> {
	let components: Vec<&str> = path.split('/').collect();

	let maybe_stream: Option<Box<dyn io::Read>> = match &components[..] {
		// Resolve internal filenames (content baked into the `tg` binary);
		// This path is always referenced, by our wrapper in `compiler.js`
		["", "__tangram__", "internal", "environment.d.ts"] => {
			let cursor = io::Cursor::new(ENVIRONMENT_DEFINITIONS);
			Some(Box::new(cursor))
		},
		["", "__tangram__", "internal", rest @ ..] => {
			bail!("Unknown internal filename: {rest:?}")
		},

		["", "__tangram__", "module", package_expr_hash, sub_path @ ..] => {
			let package_expr_hash = Hash::from_str(package_expr_hash).with_context(|| {
				format!("Failed to parse package hash in module path \"{path}\".")
			})?;
			let sub_path = sub_path.join("/");

			// Ensure the package is checked out.
			let checkout_path =
				check_out_package(builder, package_expr_hash)
					.await
					.with_context(|| {
						format!("Failed to check out package {package_expr_hash} for module \"{path}\".")
					})?;

			// Read the module from disk.
			let path_to_module = checkout_path.join(sub_path);
			match fs::File::open(&path_to_module) {
				Ok(f) => Some(Box::new(f)),
				Err(e) if e.kind() == io::ErrorKind::NotFound => None,
				Err(e) => {
					bail!("Failed to open file \"{path_to_module}\": {e}")
				},
			}
		},
		["", "__tangram__", "target-proxy", package_expr_hash, "proxy.d.ts"] => {
			let package_expr_hash = Hash::from_str(package_expr_hash).with_context(|| {
				format!("Failed to parse package hash in target-proxy path \"{path}\".")
			})?;

			// Generate a shim `.d.ts` file for the proxy import.
			let code: String = generate_code_for_proxy_import(builder, package_expr_hash).await?;
			let cursor = io::Cursor::new(code);

			Some(Box::new(cursor))
		},
		_ => {
			// Try to load the path from the filesystem
			let maybe_file = fs::File::open(path);
			match maybe_file {
				Ok(f) => Some(Box::new(f)),
				Err(e) if e.kind() == io::ErrorKind::NotFound => None,
				Err(e) => Err(e).context("Failed to open file")?,
			}
		},
	};
	Ok(maybe_stream)
}

/// Generate type definitions for proxy-imported modules.
async fn generate_code_for_proxy_import(
	builder: &Builder,
	package_expr_hash: Hash,
) -> Result<String> {
	let builder = builder.lock_shared().await?;
	let mut code = String::new();

	// Get the path to the package's entrypoint file.
	let entrypoint = builder
		.resolve_package_entrypoint_file(package_expr_hash)
		.context("Failed to get package entrypoint while generating proxy type definitions")?
		.context("Package has no entrypoint; cannot generate proxy type definitions.")?;
	let module_vfs_path = format!("/__tangram__/module/{package_expr_hash}/{entrypoint}");

	// Get the list of target names from the package's manifest.
	let package_manifest = builder
		.get_package_manifest(package_expr_hash)
		.await
		.context("Failed to get package manifest while generating proxy import type definitions")?;

	// Import the types of the upstream package.
	code.push_str(&format!(
		"import type * as upstream from \"{module_vfs_path}\";\n"
	));

	for target_name in package_manifest.targets {
		let export = if target_name == "default" {
			String::from("export default function")
		} else {
			format!("export function {target_name}")
		};

		// Generate a function definition.
		// Arguments are passed through, returns are wrapped in a `Target` expression.
		let arg_type = format!("Parameters<typeof upstream.{target_name}>");
		let return_type = format!("Tangram.Target<ReturnType<typeof upstream.{target_name}>>");
		code.push_str(&format!("{export} (...args: {arg_type}): {return_type};\n"));
	}

	Ok(code)
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolvedModuleFull {
	/// VFS path of the file to which a module was resolved.
	resolved_file_name: String,

	/// File extension of resolved_file_name, like `.ts`.
	/// This must match `resolved_file_name`.
	extension: Extension,

	/// True if `resolved_file_name` "comes from `node_modules`" from TypeScript's perspective.
	is_external_library_import: bool,
}

/// Implementation of the TypeScript `Extention` enum.
#[derive(Clone, serde::Serialize)]
enum Extension {
	#[serde(rename = ".ts")]
	Ts,
	#[serde(rename = ".tsx")]
	Tsx,
	#[serde(rename = ".d.ts")]
	Dts,
	#[serde(rename = ".js")]
	Js,
	#[serde(rename = ".jsx")]
	Jsx,
	#[serde(rename = ".json")]
	Json,
	#[serde(rename = ".tsbuildinfo")]
	TsBuildInfo,
	#[serde(rename = ".mjs")]
	Mjs,
	#[serde(rename = ".mts")]
	Mts,
	#[serde(rename = ".d.mts")]
	Dmts,
	#[serde(rename = ".cjs")]
	Cjs,
	#[serde(rename = ".cts")]
	Cts,
	#[serde(rename = ".d.cts")]
	Dcts,
}

impl Extension {
	/// Try to determine a TypeScript extension from a filename or path.
	fn parse_from_filename(file_name: &str) -> Result<Extension> {
		// Map each extension to a file suffix.
		let ext_map = [
			(Extension::Ts, ".ts"),
			(Extension::Tsx, ".tsx"),
			(Extension::Dts, ".d.ts"),
			(Extension::Js, ".js"),
			(Extension::Jsx, ".jsx"),
			(Extension::Json, ".json"),
			(Extension::TsBuildInfo, ".tsbuildinfo"),
			(Extension::Mjs, ".mjs"),
			(Extension::Mts, ".mts"),
			(Extension::Dmts, ".d.mts"),
			(Extension::Cjs, ".cjs"),
			(Extension::Cts, ".cts"),
			(Extension::Dcts, ".d.cts"),
		];

		for (extension, suffix) in ext_map {
			if file_name.ends_with(suffix) {
				return Ok(extension);
			}
		}

		bail!("Found invalid file extension for path: {file_name}");
	}
}

#[deno_core::op]
fn op_tg_resolve(
	state: Rc<RefCell<deno_core::OpState>>,
	importing_module: String,
	module_name: String,
) -> Result<ResolvedModuleFull, deno_core::error::AnyError> {
	synchify_op(state, move |builder| {
		op_tg_resolve_async(builder, importing_module, module_name)
	})
}

async fn op_tg_resolve_async(
	builder: Builder,
	importing_module: String,
	module_name: String,
) -> Result<ResolvedModuleFull, deno_core::error::AnyError> {
	// If we're dealing with an explicit VFS path, pass it through unchanged.
	if module_name.starts_with("/__tangram__/") {
		let extension = Extension::parse_from_filename(&module_name)
			.context("Explicit '/__tangram__/' import must have valid extension")?;
		return Ok(ResolvedModuleFull {
			extension,
			resolved_file_name: module_name,
			is_external_library_import: false,
		});
	}

	// If we're dealing with a relative import, just modify the import subpath.
	if module_name.starts_with("./") || module_name.starts_with("../") {
		let importing_module_path = Utf8PathBuf::from(importing_module);
		let extension = Extension::parse_from_filename(&module_name)
			.context("Relative import did not have a file extension.")?;

		// TODO: Handle edge cases by normalizing `../` and `./` properly.
		let resolved_file_name = importing_module_path
			.parent()
			.context("Importing module path has no parent directory")?
			.join(&module_name)
			.as_str()
			.to_owned();

		return Ok(ResolvedModuleFull {
			resolved_file_name,
			extension,
			is_external_library_import: false,
		});
	}

	// Here, we're dealing with a `tangram:packagename/[optional-subpath].ext` import.

	let (_tangram_prefix, name_and_maybe_subpath) = module_name.split_once("tangram:").context(
		"Module name does not start with either `tangram:` or relative path (`./` or `../`).",
	)?;

	let name_and_subpath_components: Vec<&str> = name_and_maybe_subpath.split('/').collect();

	// Parse the module name and subpath.
	let (module_name, subpath) = match &name_and_subpath_components[..] {
		[module_name] => (module_name, None),
		[module_name, subpath_components @ ..] => {
			let subpath = Utf8PathBuf::from(subpath_components.join("/"));
			(module_name, Some(subpath))
		},
		_ => bail!("Invalid module name and subpath: {name_and_maybe_subpath:?}"),
	};

	// Get the dependencies of the module
	//   If the referrer is a plain file: traverse and parse lockfile
	//   If the referrer comes from a package-expression: get it from the expression
	let referrer_path = Utf8PathBuf::from(&importing_module);
	let referrer_dependencies: BTreeMap<Arc<str>, Hash> = if is_special_path(&referrer_path) {
		// `referrer_path` is a special path, representing a package expression.
		// We parse the path, extracting the hash of the importing package-expression.
		let referrer_path_components: Vec<&str> = importing_module.split('/').collect();
		match &referrer_path_components[..] {
			["", "__tangram__", "target-proxy" | "module", hash_str, ..] => {
				let hash = Hash::from_str(hash_str)
					.context("Could not parse hash of referring module from its path.")?;

				// Get the package expression from the builder
				let package_expr: expression::Package = builder
					.lock_shared()
					.await?
					.get_expression_local(hash)
					.context("Failed to get package expression for referring package")?
					.into_package()
					.context("Referrer path did not contain the hash of a package expression")?;

				package_expr.dependencies
			},

			_ => bail!("Unexpected VFS path found as module referrer while resolving."),
		}
	} else {
		// `referrer_path` is a disk path, so we need to find a lockfile for it.
		let lockfile = load_governing_lockfile_for_module_file(&referrer_path)
			.await
			.with_context(|| {
				format!("Failed to read lockfile for module \"{referrer_path}\" while resolving imports.")
			})?
			.with_context(|| {
				format!("Could not find governing lockfile for module \"{referrer_path}\" while resolving imports.")
			})?;

		// Extract the dependency map from the lockfile.
		match lockfile {
			Lockfile::V1(lockfile) => lockfile
				.dependencies
				.into_iter()
				.map(|(name, dep)| (Arc::from(name), dep.hash))
				.collect(),
		}
	};

	// TODO: Use unified resolution with the runtime.
	//       Currently, we can't use `Compiler::resolve`, because the referring package is most
	//       likely not checked in. Because of this, we can't construct a valid referrer URL.
	//       `Compiler::resolve` must be refactored to take a dependency map.

	// Look up the specifier's package name in the referrer's dependencies.
	let specifier_package_hash = *referrer_dependencies.get(*module_name).ok_or_else(|| {
		anyhow!(
			r#"Expected the referrer's package dependencies to contain the specifier's package name."#
		)
	})?;

	// TODO: check that the extension is valid

	debug!(
		module_name,
		importing_module,
		%specifier_package_hash,
		?subpath,
		"Resolved module"
	);

	// Create a VFS path to the right file.
	let vfs_path = if let Some(subpath) = subpath {
		format!("/__tangram__/module/{specifier_package_hash}/{subpath}")
	} else {
		// TODO: resolve the entrypoint file here instead.
		format!("/__tangram__/target-proxy/{specifier_package_hash}/proxy.d.ts")
	};

	trace!(importing_module, module_name, vfs_path, "op_tg_resolve");

	let extension = Extension::parse_from_filename(&vfs_path)
		.context("Failed to determine file extension while resolving module")?;

	Ok(ResolvedModuleFull {
		resolved_file_name: vfs_path,
		extension,

		// TODO: Figure out how to integrate with the TypeScript module cache.
		//       Generate "package names" with hashes, and let TS cache the types.
		is_external_library_import: false,
	})
}

/// For modules that have not been checked into the store, load the closest lockfile found by
/// traversing up the directory hierarchy.
pub async fn load_governing_lockfile_for_module_file(path: &Utf8Path) -> Result<Option<Lockfile>> {
	// We can only look for the lockfile when we're dealing with a module path that's *actually* a
	// file in the user's working directory. This indicates a bug, so we panic if this invariant
	// isn't held.
	assert!(
		!is_special_path(path),
		"Invalid search for lockfile in checked-in VFS path."
	);

	// Look in each ancestor directory for a `tangram.lock` file.
	for dir_to_check in path.ancestors() {
		let possible_lockfile_path = dir_to_check.join("tangram.lock");
		if let Ok(meta) = tokio::fs::metadata(&possible_lockfile_path).await {
			if meta.is_file() {
				// Here, we've found a lockfile. Load it and return it.
				let lockfile_path = possible_lockfile_path;
				let lockfile_text = tokio::fs::read(&lockfile_path).await.with_context(|| {
					format!(
						r#"Failed to read lockfile at "{lockfile_path}" for module at "{path}"."#
					)
				})?;
				let lockfile: Lockfile =
					serde_json::from_slice(&lockfile_text).with_context(|| {
						format!("Failed to parse lockfile JSON at \"{lockfile_path}\"")
					})?;
				return Ok(Some(lockfile));
			}
		}
	}

	// We've reached the root, and still no lockfile. Return none.
	Ok(None)
}

#[deno_core::op]
fn op_tg_read_file(
	state: Rc<RefCell<deno_core::OpState>>,
	file_name: String,
) -> Result<Option<String>, deno_core::error::AnyError> {
	synchify_op(state, move |builder| {
		op_tg_read_file_async(builder, file_name)
	})
}

async fn op_tg_read_file_async(
	builder: Builder,
	file_name: String,
) -> Result<Option<String>, deno_core::error::AnyError> {
	let file = vfs_read_file(&builder, &file_name).await?;
	match file {
		None => {
			trace!(file_name, exists = false, "op_tg_read_file");
			Ok(None)
		},
		Some(mut reader) => {
			let mut contents = String::new();
			reader
				.read_to_string(&mut contents)
				.context("Failed to read file contents")?;
			trace!(
				file_name,
				exists = true,
				content_len = contents.len(),
				"op_tg_read_file"
			);
			Ok(Some(contents))
		},
	}
}

#[deno_core::op]
fn op_tg_file_exists(
	state: Rc<RefCell<deno_core::OpState>>,
	file_name: String,
) -> Result<bool, deno_core::error::AnyError> {
	synchify_op(state, move |builder| {
		op_tg_file_exists_async(builder, file_name)
	})
}

async fn op_tg_file_exists_async(
	builder: Builder,
	file_name: String,
) -> Result<bool, deno_core::error::AnyError> {
	let file = vfs_read_file(&builder, &file_name).await?;
	match file {
		None => {
			trace!(file_name, exists = false, "op_tg_file_exists");
			Ok(false)
		},
		Some(_) => {
			trace!(file_name, exists = true, "op_tg_file_exists");
			Ok(true)
		},
	}
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
fn op_tg_console_error(args: Vec<serde_json::Value>) -> Result<(), deno_core::error::AnyError> {
	let mut msg = String::new();
	let len = args.len();
	for (i, arg) in args.into_iter().enumerate() {
		write!(msg, "{arg:#}").unwrap();
		if i != len - 1 {
			write!(msg, " ").unwrap();
		}
	}
	error!("{}", msg);
	Ok(())
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
fn op_tg_console_log(args: Vec<serde_json::Value>) -> Result<(), deno_core::error::AnyError> {
	let mut msg = String::new();
	let len = args.len();
	for (i, arg) in args.into_iter().enumerate() {
		write!(msg, "{arg:#}").unwrap();
		if i != len - 1 {
			write!(msg, " ").unwrap();
		}
	}
	trace!("console.log: {msg}");
	Ok(())
}

/// Make an operation synchronous by blocking the thread.
#[allow(clippy::needless_pass_by_value)]
fn synchify_op<T, F, Fut>(
	state: Rc<RefCell<deno_core::OpState>>,
	f: F,
) -> Result<T, deno_core::error::AnyError>
where
	T: 'static + Send,
	F: FnOnce(Builder) -> Fut + Send + 'static,
	Fut: 'static + Send + Future<Output = Result<T, deno_core::error::AnyError>>,
{
	let state = {
		let state = state.borrow();
		let state = state.borrow::<Arc<OpState>>();
		Arc::clone(state)
	};

	std::thread::spawn(move || state.main_runtime_handle.block_on(f(state.builder.clone())))
		.join()
		.unwrap()
}
