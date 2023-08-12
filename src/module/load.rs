use super::Module;
use crate::{
	block::Block,
	error::{Result, WrapErr},
	instance::Instance,
	package::Package,
};
use include_dir::include_dir;

const TANGRAM_D_TS: &str = include_str!(concat!(
	env!("CARGO_MANIFEST_DIR"),
	"/src/global/tangram.d.ts"
));
const LIB: include_dir::Dir = include_dir!("$CARGO_MANIFEST_DIR/assets/lib");

impl Module {
	/// Load the module.
	pub async fn load(&self, tg: &Instance) -> Result<String> {
		match self {
			// Load a library module.
			Self::Library(module) => {
				let path = module.module_path.to_string();
				let text = match path.as_str() {
					"tangram.d.ts" => TANGRAM_D_TS,
					_ => LIB
						.get_file(&path)
						.wrap_err_with(|| {
							format!(r#"Could not find a library module with the path "{path}"."#)
						})?
						.contents_utf8()
						.wrap_err("Failed to read the file as UTF-8.")?,
				};
				Ok(text.to_owned())
			},

			// Load a module from a document.
			Self::Document(document) => document.text(tg).await,

			// Load a module from a package.
			Self::Normal(module) => {
				// Get the package.
				let package = Package::with_block(tg, Block::with_id(module.package)).await?;

				// Load the module.
				let directory = package.artifact().as_directory().unwrap();
				let entry = directory.get(tg, &module.module_path).await?;
				let file = entry.into_file().wrap_err("Expected a file.")?;
				let text = file.contents(tg).await?.text(tg).await?;

				Ok(text)
			},
		}
	}
}
