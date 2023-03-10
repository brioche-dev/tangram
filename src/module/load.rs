use super::{identifier, Identifier};
use crate::{
	artifact::{self, Artifact},
	error::{bail, Context, Result},
	hash, os, package,
	path::{self, Path},
	Instance,
};
use include_dir::include_dir;
use indoc::formatdoc;
use itertools::Itertools;
use tokio::io::AsyncReadExt;

impl Instance {
	/// Load a module with the given module identifier.
	pub async fn load_module(&self, module_identifier: &Identifier) -> Result<String> {
		match module_identifier {
			Identifier::Normal(identifier) => self.load_normal_module(identifier).await,
			Identifier::Artifact(identifier) => self.load_artifact_module(identifier).await,
			Identifier::Lib(identifier) => load_lib_module(identifier),
		}
	}
}

const TANGRAM_D_TS: &str = include_str!("../global/tangram.d.ts");
const LIB: include_dir::Dir = include_dir!("$CARGO_MANIFEST_DIR/assets/lib");

fn load_lib_module(module_identifier: &identifier::Lib) -> Result<String> {
	// Collect the path components.
	let path = module_identifier
		.path
		.components
		.iter()
		.map(path::Component::as_str)
		.join("/");

	// Get the module text.
	let text = match path.as_str() {
		"tangram.d.ts" => TANGRAM_D_TS,
		_ => LIB
			.get_file(&path)
			.with_context(|| format!(r#"Could not find a lib with the path "{path}"."#))?
			.contents_utf8()
			.context("Failed to read the file as UTF-8.")?,
	};

	Ok(text.to_owned())
}

impl Instance {
	#[allow(clippy::unused_async)]
	async fn load_artifact_module(&self, identifier: &identifier::Artifact) -> Result<String> {
		// Get the artifact hash.
		let (ty, artifact_hash) = match &identifier.source {
			identifier::Source::Path(package_path) => {
				// Get the path.
				let path = package_path.join(identifier.path.to_string());

				// Use a zero hash for artifact modules whose source is a path because the code will never run.
				let artifact_hash = artifact::Hash(hash::Hash::zero());

				// Get the type.
				let metadata = tokio::fs::symlink_metadata(path).await?;
				let ty = if metadata.is_dir() {
					"tg.Directory"
				} else if metadata.is_file() {
					"tg.File"
				} else if metadata.is_symlink() {
					"tg.Symlink"
				} else {
					bail!("The path must point to a directory, file, or symlink.")
				};

				(ty, artifact_hash)
			},

			identifier::Source::Instance(package_instance_hash) => {
				// Get the package.
				let package_instance = self.get_package_instance_local(*package_instance_hash)?;
				let package_hash = package_instance.package_hash;

				// Get the entry.
				let artifact_hash = self.directory_get(package_hash, &identifier.path).await?;

				// Get the artifact.
				let artifact = self.get_artifact_local(artifact_hash)?;

				// Get the type.
				let ty = match artifact {
					Artifact::Directory(_) => "tg.Directory",
					Artifact::File(_) => "tg.File",
					Artifact::Symlink(_) => "tg.Symlink",
					Artifact::Reference(_) => "tg.Reference",
				};

				(ty, artifact_hash)
			},
		};

		// Generate the code.
		let code = formatdoc!(
			r#"
				export default await ({ty} as any).fromHash("{artifact_hash}") as {ty}
			"#
		);

		Ok(code)
	}
}

impl Instance {
	async fn load_normal_module(&self, identifier: &identifier::Normal) -> Result<String> {
		match &identifier.source {
			identifier::Source::Path(package_path) => {
				self.load_normal_module_from_path(package_path, &identifier.path)
					.await
			},
			identifier::Source::Instance(package_instance_hash) => {
				self.load_normal_module_from_instance(*package_instance_hash, &identifier.path)
					.await
			},
		}
	}

	async fn load_normal_module_from_path(
		&self,
		package_path: &os::Path,
		path: &Path,
	) -> Result<String> {
		// Construct the path to the module.
		let path = package_path.join(path.to_string());

		// Read the file from disk.
		let text = tokio::fs::read_to_string(&path).await.with_context(|| {
			let path = path.display();
			format!(r#"Failed to load the module at path "{path}"."#)
		})?;

		Ok(text)
	}

	async fn load_normal_module_from_instance(
		&self,
		package_instance_hash: package::instance::Hash,
		path: &Path,
	) -> Result<String> {
		// Get the package.
		let package_instance = self.get_package_instance_local(package_instance_hash)?;
		let package_hash = package_instance.package_hash;

		// Get the module artifact in the package.
		let module_artifact_hash = self.directory_get(package_hash, path).await?;
		let module_artifact = self.get_artifact_local(module_artifact_hash)?;
		let module_file = module_artifact.into_file().context("Expected a file.")?;

		// Read the module.
		let mut text = String::new();
		self.get_blob(module_file.blob_hash)
			.await?
			.read_to_string(&mut text)
			.await?;

		Ok(text)
	}
}
