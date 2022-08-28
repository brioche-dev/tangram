use crate::{
	artifact::Artifact, hash::Hash, lockfile::Lockfile, manifest::Manifest, object::ObjectHash,
	server::Server,
};
use anyhow::{anyhow, bail, ensure, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use fnv::FnvHashMap;
use futures::FutureExt;
use indoc::formatdoc;
use std::{
	pin::Pin,
	sync::{Arc, Mutex},
};
use url::Url;

pub const TANGRAM_SCHEME: &str = "tangram";
pub const TANGRAM_MODULE_SCHEME: &str = "tangram-module";
pub const TANGRAM_TARGET_PROXY_SCHEME: &str = "tangram-target-proxy";

pub struct ModuleLoader {
	server: Arc<Server>,
	main_runtime_handle: tokio::runtime::Handle,
	lockfile_cache: Arc<Mutex<FnvHashMap<Hash, Lockfile>>>,
}

impl ModuleLoader {
	/// Create a new module loader.
	pub fn new(server: Arc<Server>, main_runtime_handle: tokio::runtime::Handle) -> ModuleLoader {
		ModuleLoader {
			server,
			main_runtime_handle,
			lockfile_cache: Arc::new(Mutex::new(FnvHashMap::default())),
		}
	}

	/// Add a lockfile to the module loader's lockfile cache.
	pub fn add_lockfile(&self, lockfile: Lockfile) -> Hash {
		let hash = Hash::new(serde_json::to_vec(&lockfile).unwrap());
		self.lockfile_cache.lock().unwrap().insert(hash, lockfile);
		hash
	}
}

impl deno_core::ModuleLoader for ModuleLoader {
	fn resolve(
		&self,
		specifier: &str,
		referrer: &str,
		_is_main: bool,
	) -> Result<deno_core::ModuleSpecifier> {
		let server = Arc::clone(&self.server);
		let lockfile_cache = Arc::clone(&self.lockfile_cache);

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
				futures::executor::block_on(resolve_tangram(
					server,
					lockfile_cache,
					specifier,
					referrer,
				))?
			},

			// Resolve a specifier with the tangram module scheme.
			TANGRAM_MODULE_SCHEME => {
				futures::executor::block_on(resolve_tangram_module(specifier, referrer))?
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
		let server = Arc::clone(&self.server);
		let main_runtime_handle = self.main_runtime_handle.clone();
		let lockfile_cache = Arc::clone(&self.lockfile_cache);
		let referrer = maybe_referrer;
		let specifier = module_specifier.clone();
		main_runtime_handle
			.spawn(async move {
				match specifier.scheme() {
					// Load a module with the tangram module scheme.
					TANGRAM_MODULE_SCHEME => load_tangram_module(server, specifier, referrer).await,

					// Load a module with the tangram target proxy scheme.
					TANGRAM_TARGET_PROXY_SCHEME => {
						load_tangram_target_proxy(server, lockfile_cache, specifier, referrer).await
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
	server: Arc<Server>,
	lockfile_cache: Arc<Mutex<FnvHashMap<Hash, Lockfile>>>,
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
	let referrer_package_hash: ObjectHash = domain
		.parse()
		.with_context(|| "Failed to parse referrer domain.")?;
	let referrer_package = Artifact::with_hash(referrer_package_hash)
		.await
		.with_context(|| "Failed to retrieve referrer artifact.")?
		.ok_or_else(|| anyhow!("Failed to find referrer package."))?;

	// Get the lockfile hash from the referrer.
	let referrer_lockfile_hash = if let Some(referrer_lockfile_hash) =
		referrer.query_pairs().find_map(|(key, value)| {
			if key == "lockfile_hash" {
				Some(value)
			} else {
				None
			}
		}) {
		let referrer_lockfile_hash = referrer_lockfile_hash
			.parse()
			.with_context(|| "Failed to parse lockfile hash.")?;
		Some(referrer_lockfile_hash)
	} else {
		None
	};

	// Retrieve the referrer's lockfile from the cache or from the package.
	let referrer_lockfile = if let Some(referrer_lockfile_hash) = referrer_lockfile_hash {
		let lockfile_cache = lockfile_cache.lock().unwrap();
		let referrer_lockfile: Lockfile = lockfile_cache
			.get(&referrer_lockfile_hash)
			.ok_or_else(|| {
				anyhow!(r#"Failed to find lockfile with hash {referrer_lockfile_hash}."#)
			})?
			.clone();
		referrer_lockfile
	} else {
		// Create a fragment for the referrer's package.
		let referrer_fragment = server.create_fragment(&referrer_package).await?;
		let referrer_fragment_path = referrer_fragment.path();

		// Read the referrer's lockfile.
		let referrer_lockfile_path = referrer_fragment_path.join("tangram.lock");
		let referrer_lockfile = tokio::fs::read(&referrer_lockfile_path).await?;
		let referrer_lockfile: Lockfile = serde_json::from_slice(&referrer_lockfile)?;
		referrer_lockfile
	};
	let referrer_lockfile = referrer_lockfile.as_v1().unwrap();

	// Get the specifier's package name and sub path.
	let specifier_path = Utf8Path::new(specifier.path());
	let specifier_package_name = specifier_path.components().next().unwrap().as_str();
	let specifier_sub_path = if specifier_path.components().count() > 1 {
		Some(specifier_path.components().skip(1).collect::<Utf8PathBuf>())
	} else {
		None
	};

	// Retrieve the specifier's entry in the referrer's lockfile.
	let lockfile_entry = referrer_lockfile
		.dependencies
		.get(specifier_package_name)
		.ok_or_else(|| anyhow!(r#"Could not find package "{specifier_package_name}"."#))?;
	let specifier_package = lockfile_entry.hash;

	// Get the specifier's lockfile.
	let specifier_lockfile = lockfile_entry
		.dependencies
		.as_ref()
		.map(|dependencies| Lockfile::new_v1(dependencies.clone()));

	// Add the specifier's lockfile to the lockfile cache.
	let specifier_lockfile_hash = if let Some(specifier_lockfile) = specifier_lockfile {
		let specifier_lockfile_json = serde_json::to_string(&specifier_lockfile)?;
		let specifier_lockfile_hash = Hash::new(&specifier_lockfile_json);
		lockfile_cache
			.lock()
			.unwrap()
			.insert(specifier_lockfile_hash, specifier_lockfile);
		Some(specifier_lockfile_hash)
	} else {
		None
	};

	// Compute the URL to resolve to.
	let mut url = if let Some(specifier_sub_path) = specifier_sub_path {
		format!("{TANGRAM_MODULE_SCHEME}://{specifier_package}/{specifier_sub_path}")
	} else {
		format!("{TANGRAM_TARGET_PROXY_SCHEME}://{specifier_package}")
	};
	if let Some(specifier_lockfile_hash) = specifier_lockfile_hash {
		url.push_str(&format!("?lockfile_hash={specifier_lockfile_hash}"));
	}
	let url = Url::parse(&url).unwrap();

	Ok(url)
}

#[allow(clippy::unused_async)]
async fn resolve_tangram_module(
	specifier: deno_core::ModuleSpecifier,
	_referrer: Option<deno_core::ModuleSpecifier>,
) -> Result<deno_core::ModuleSpecifier> {
	Ok(specifier)
}

async fn load_tangram_module(
	server: Arc<Server>,
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
	let specifier_artifact: Artifact = domain.parse()?;

	// Create a fragment for the specifier's package.
	let fragment = server.create_fragment(&specifier_artifact).await?;
	let fragment_path = fragment.path();

	// Get the path from the specifier.
	let specifier_path = Utf8Path::new(specifier.path());
	let specifier_path = specifier_path
		.strip_prefix("/")
		.with_context(|| anyhow!("The specifier must have a leading slash."))?;

	// Read the module's code.
	let module_path = fragment_path.join(specifier_path);
	let code = tokio::fs::read(&module_path).await.with_context(|| {
		anyhow!(
			r#"Failed to read file at path "{}"."#,
			module_path.display(),
		)
	})?;
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

async fn load_tangram_target_proxy(
	server: Arc<Server>,
	lockfile_cache: Arc<Mutex<FnvHashMap<Hash, Lockfile>>>,
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
	let package: Artifact = domain.parse()?;

	// Create a fragment for the specifier's package.
	let fragment = server.create_fragment(&package).await?;
	let fragment_path = fragment.path();

	// Read the specifier's manifest.
	let manifest = tokio::fs::read(&fragment_path.join("tangram.json")).await?;
	let manifest: Manifest = serde_json::from_slice(&manifest)?;

	// Get the lockfile hash from the specifier.
	let lockfile_hash = specifier.query_pairs().find_map(|(key, value)| {
		if key == "lockfile_hash" {
			Some(value)
		} else {
			None
		}
	});
	let lockfile_hash: Option<Hash> = if let Some(lockfile_hash) = lockfile_hash {
		let referrer_lockfile_hash = lockfile_hash
			.parse()
			.with_context(|| "Failed to parse the lockfile hash.")?;
		Some(referrer_lockfile_hash)
	} else {
		None
	};

	// Get the lockfile.
	let lockfile: Option<Lockfile> = if let Some(lockfile_hash) = lockfile_hash {
		let lockfile = lockfile_cache
			.lock()
			.unwrap()
			.get(&lockfile_hash)
			.ok_or_else(|| {
				anyhow!(
					r#"Failed to get a lockfile from the lockfile cache with the hash "{lockfile_hash}"."#
				)
			})?
			.clone();
		Some(lockfile)
	} else {
		None
	};

	// Generate the code for the target proxy module.
	let mut code = String::new();
	let lockfile_json = serde_json::to_string(&lockfile)?;
	let package_json = serde_json::to_string(&package)?;
	code.push_str(&formatdoc!(
		r#"
			let lockfile = {lockfile_json};
		"#
	));
	code.push('\n');
	for target_name in manifest.targets {
		code.push_str(&formatdoc!(
			r#"
				export let {target_name} = (...args) => {{
					return Tangram.target({{
						lockfile,
						package: {package_json},
						name: "{target_name}",
						args,
					}});
				}}
			"#,
		));
		code.push('\n');
	}

	Ok(deno_core::ModuleSource {
		code: code.into_bytes().into_boxed_slice(),
		module_type: deno_core::ModuleType::JavaScript,
		module_url_specified: specifier.to_string(),
		module_url_found: specifier.to_string(),
	})
}
