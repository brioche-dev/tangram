use crate::{
	constants::ROOT_MODULE_FILE_NAME,
	error::{return_error, Error, Result, WrapErr},
	package,
	path::Path,
	util::fs,
};
use url::Url;

/// A unique identifier for a module.
#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct Identifier {
	/// The module's source.
	pub source: Source,

	/// The module's path.
	pub path: Path,
}

/// The source for a module.
#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(tag = "kind", content = "value")]
pub enum Source {
	/// A module in the library.
	#[serde(rename = "lib")]
	Lib,

	/// A module in a package.
	#[serde(rename = "package")]
	Package(package::Identifier),

	/// A module in a package instance.
	#[serde(rename = "package_instance")]
	PackageInstance(package::instance::Hash),
}

impl Identifier {
	#[must_use]
	pub fn for_root_module_in_package(package_identifier: package::Identifier) -> Identifier {
		Identifier {
			source: Source::Package(package_identifier),
			path: ROOT_MODULE_FILE_NAME.parse().unwrap(),
		}
	}

	#[must_use]
	pub fn for_root_module_in_package_instance(
		package_instance_hash: package::instance::Hash,
	) -> Identifier {
		Identifier {
			source: Source::PackageInstance(package_instance_hash),
			path: ROOT_MODULE_FILE_NAME.parse().unwrap(),
		}
	}
}

impl Identifier {
	pub async fn for_path(path: &fs::Path) -> Result<Identifier> {
		// Find the package path by searching the path's ancestors for a root module.
		let mut found = false;
		let mut package_path = path.to_owned();
		while package_path.pop() {
			if crate::util::fs::exists(&package_path.join(ROOT_MODULE_FILE_NAME)).await? {
				found = true;
				break;
			}
		}
		if !found {
			let path = path.display();
			return_error!(r#"Could not find the package for path "{path}"."#,);
		}

		// Get the module path by stripping the package path.
		let module_path: Path = path
			.strip_prefix(&package_path)
			.unwrap()
			.to_owned()
			.into_os_string()
			.into_string()
			.ok()
			.wrap_err("The module path was not valid UTF-8.")?
			.parse()
			.wrap_err("The module path was not a valid path.")?;

		// Create the source.
		let source = Source::Package(package::Identifier::Path(package_path));

		// Create the module identifier.
		let module_identifier = Identifier {
			source,
			path: module_path,
		};

		Ok(module_identifier)
	}
}

impl From<Identifier> for Url {
	fn from(value: Identifier) -> Self {
		// Serialize and encode the identifier.
		let data = hex::encode(serde_json::to_string(&value).unwrap());

		// Create the URL.
		format!("tangram:{data}").parse().unwrap()
	}
}

impl TryFrom<Url> for Identifier {
	type Error = Error;

	fn try_from(value: Url) -> Result<Self, Self::Error> {
		// Ensure the scheme is "tangram".
		if value.scheme() != "tangram" {
			return_error!("The URL has an invalid scheme.");
		}

		// Decode.
		let data = hex::decode(value.path())
			.map_err(Error::other)
			.wrap_err("Failed to deserialize the path as hex.")?;

		// Deserialize.
		let identifier = serde_json::from_slice(&data)
			.map_err(Error::other)
			.wrap_err("Failed to deserialize the identifier.")?;

		Ok(identifier)
	}
}

impl std::fmt::Display for Identifier {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let url: Url = self.clone().into();
		write!(f, "{url}")?;
		Ok(())
	}
}

impl std::str::FromStr for Identifier {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let url: Url = s.parse().map_err(Error::other)?;
		let module_identifier = url.try_into()?;
		Ok(module_identifier)
	}
}

impl From<Identifier> for String {
	fn from(value: Identifier) -> Self {
		value.to_string()
	}
}

impl TryFrom<String> for Identifier {
	type Error = Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
	}
}
