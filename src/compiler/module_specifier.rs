use std::str::FromStr;

use anyhow::{bail, Context};
use camino::Utf8PathBuf;

use crate::package_specifier::PackageSpecifier;

use super::module_identifier::TANGRAM_SCHEME;

/// A `ModuleSpecifier` represents an Tangram TypeScript module import specifier. In the expression `import * as std from "tangram:std"`, the module specifier is `tangram:std`.
#[derive(Debug, Clone)]
pub enum ModuleSpecifier {
	/// `ModuleSpecifier::Path` represents a relative path import in the current package, such as `import "./file.tg"`.
	Path { module_path: Utf8PathBuf },

	/// `ModuleSpecifier::Package` represents an import from another package, such as `import "tangram:std"`. See [`PackageSpecifier`] for more details about the exact semantics of a package specifier.
	Package(PackageSpecifier),
}

impl FromStr for ModuleSpecifier {
	type Err = anyhow::Error;

	fn from_str(specifier: &str) -> Result<Self, Self::Err> {
		// A path speciifer starts with a `/`, `./`, or `../`.
		let first_component = specifier.split('/').next().unwrap();
		let is_path_specifier = matches!(first_component, "" | "." | "..");

		if is_path_specifier {
			// Interpret the specifier as a path.
			let module_path = Utf8PathBuf::from_str(specifier)?;
			Ok(ModuleSpecifier::Path { module_path })
		} else {
			// Parse the package specifier as a URL.
			let url = url::Url::parse(specifier).with_context(|| {
				format!("Module specifier {specifier:?} should be a valid URL or relative path.")
			})?;

			match url.scheme() {
				TANGRAM_SCHEME => {
					// Parse everything after the `tangram:` scheme as a package specifier.
					let package_specifier =
						url.path().parse().with_context(|| {
							format!("Module specifier {specifier:?} should be a valid package specifier.")
						})?;
					Ok(ModuleSpecifier::Package(package_specifier))
				},
				_ => bail!("Unknown schema for module specifier {specifier:?}."),
			}
		}
	}
}
