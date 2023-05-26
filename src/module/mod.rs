pub use self::import::Import;
use crate::{
	document::Document,
	error::{return_error, Error, Result, WrapErr},
	package,
	path::Subpath,
};
use url::Url;

pub mod analyze;
pub mod error;
pub mod import;
pub mod load;
pub mod parse;
pub mod resolve;
pub mod transpile;
mod version;

/// A module.
#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Module {
	/// A library module.
	Library(Library),

	/// A document module.
	Document(Document),

	/// A normal module.
	Normal(Normal),
}

#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct Library {
	/// The module's path.
	pub module_path: Subpath,
}

#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct Normal {
	/// The module's package hash.
	pub package_hash: package::Hash,

	/// The module's path.
	pub module_path: Subpath,
}

impl From<Module> for Url {
	fn from(value: Module) -> Self {
		// Serialize and encode the module.
		let data = hex::encode(serde_json::to_string(&value).unwrap());

		let path = match value {
			Module::Library(library) => format!("/{}", library.module_path),
			Module::Document(document) => format!(
				"/{}/{}",
				document.package_path.display(),
				document.module_path
			),
			Module::Normal(normal) => format!("/{}", normal.module_path),
		};

		// Create the URL.
		format!("tangram://{data}{path}").parse().unwrap()
	}
}

impl TryFrom<Url> for Module {
	type Error = Error;

	fn try_from(value: Url) -> Result<Self, Self::Error> {
		// Ensure the scheme is "tangram".
		if value.scheme() != "tangram" {
			return_error!("The URL has an invalid scheme.");
		}

		// Get the domain.
		let data = value.domain().wrap_err("The URL must have a domain.")?;

		// Decode.
		let data = hex::decode(data)
			.map_err(Error::other)
			.wrap_err("Failed to deserialize the path as hex.")?;

		// Deserialize.
		let module = serde_json::from_slice(&data)
			.map_err(Error::other)
			.wrap_err("Failed to deserialize the module.")?;

		Ok(module)
	}
}

impl std::fmt::Display for Module {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let url: Url = self.clone().into();
		write!(f, "{url}")?;
		Ok(())
	}
}

impl std::str::FromStr for Module {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let url: Url = s.parse().map_err(Error::other)?;
		let module = url.try_into()?;
		Ok(module)
	}
}

impl From<Module> for String {
	fn from(value: Module) -> Self {
		value.to_string()
	}
}

impl TryFrom<String> for Module {
	type Error = Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
	}
}
