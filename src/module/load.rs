use super::{identifier::Source, Identifier};
use crate::{
	error::{Result, WrapErr},
	package,
	path::Path,
	util::fs,
	Instance,
};
use include_dir::include_dir;

impl Instance {
	/// Load a module with the given module identifier.
	pub async fn load_module(&self, module_identifier: &Identifier) -> Result<String> {
		match &module_identifier.source {
			Source::Lib => load_module_from_lib(&module_identifier.path),
			Source::Path(package_path) => {
				self.load_module_from_path(package_path, &module_identifier.path)
					.await
			},
			Source::Instance(package_instance_hash) => {
				self.load_module_from_instance(*package_instance_hash, &module_identifier.path)
					.await
			},
		}
	}
}

const TANGRAM_D_TS: &str = include_str!("../global/tangram.d.ts");
const LIB: include_dir::Dir = include_dir!("$CARGO_MANIFEST_DIR/assets/lib");

fn load_module_from_lib(path: &Path) -> Result<String> {
	// Get the module text.
	let path = path.to_string();
	let text = match path.as_str() {
		"tangram.d.ts" => TANGRAM_D_TS,
		_ => LIB
			.get_file(&path)
			.wrap_err_with(|| format!(r#"Could not find a lib with the path "{path}"."#))?
			.contents_utf8()
			.wrap_err("Failed to read the file as UTF-8.")?,
	};

	Ok(text.to_owned())
}

impl Instance {
	async fn load_module_from_path(&self, package_path: &fs::Path, path: &Path) -> Result<String> {
		// Construct the path to the module.
		let path = package_path.join(path.to_string());

		// Read the file from disk.
		let text = tokio::fs::read_to_string(&path).await.wrap_err_with(|| {
			let path = path.display();
			format!(r#"Failed to load the module at path "{path}"."#)
		})?;

		Ok(text)
	}

	async fn load_module_from_instance(
		&self,
		package_instance_hash: package::instance::Hash,
		path: &Path,
	) -> Result<String> {
		// Get the package.
		let package_instance = self.get_package_instance_local(package_instance_hash)?;
		let package = self.get_artifact_local(package_instance.package_hash)?;

		// Get the module.
		let module = package
			.as_directory()
			.wrap_err("Expected a package to be a directory.")?
			.get(self, path)
			.await?
			.into_file()
			.wrap_err("Expected a file.")?;

		// Read the module.
		let text = module.read_to_string(self).await?;

		Ok(text)
	}
}
