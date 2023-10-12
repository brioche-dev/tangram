use super::{document, Module};
use crate::{Client, Package, Result, WrapErr};
use include_dir::include_dir;

const TANGRAM_D_TS: &str = include_str!(concat!(
	env!("CARGO_MANIFEST_DIR"),
	"/../runtime/src/js/tangram.d.ts"
));
const LIB: include_dir::Dir = include_dir!("$CARGO_MANIFEST_DIR/../lsp/src/lib");

impl Module {
	/// Load the module.
	pub async fn load(
		&self,
		client: &dyn Client,
		document_store: Option<&document::Store>,
	) -> Result<String> {
		match self {
			// Load a library module.
			Self::Library(module) => {
				let path = module.path.to_string();
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
			Self::Document(document) => document.text(document_store.unwrap()).await,

			// Load a module from a package.
			Self::Normal(module) => {
				// Get the package.
				let package = Package::with_id(module.package_id);

				// Load the module.
				let directory = package
					.artifact(client)
					.await?
					.try_unwrap_directory_ref()
					.unwrap();
				let entry = directory.get(client, &module.path).await?;
				let file = entry
					.try_unwrap_file_ref()
					.ok()
					.wrap_err("Expected a file.")?;
				let text = file.contents(client).await?.text(client).await?;

				Ok(text)
			},
		}
	}
}
