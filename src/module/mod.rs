pub use self::specifier::Specifier;
use crate::{
	document::Document,
	error::{return_error, Error, Result, WrapErr},
	package,
	path::Path,
};
use url::Url;

pub mod dependency;
pub mod load;
mod path;
pub mod resolve;
pub mod specifier;
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
	pub module_path: Path,
}

#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct Normal {
	/// The module's package instance hash.
	pub package_instance_hash: package::instance::Hash,

	/// The module's path.
	pub module_path: Path,
}

impl From<Module> for Url {
	fn from(value: Module) -> Self {
		// Serialize and encode the identifier.
		let data = hex::encode(serde_json::to_string(&value).unwrap());

		// Create the URL.
		format!("tangram:{data}.tg").parse().unwrap()
	}
}

impl TryFrom<Url> for Module {
	type Error = Error;

	fn try_from(value: Url) -> Result<Self, Self::Error> {
		// Ensure the scheme is "tangram".
		if value.scheme() != "tangram" {
			return_error!("The URL has an invalid scheme.");
		}

		// Strip the ".tg" extension.
		let path = value
			.path()
			.strip_suffix(".tg")
			.wrap_err("The URL has an invalid extension.")?;

		// Decode.
		let data = hex::decode(path)
			.map_err(Error::other)
			.wrap_err("Failed to deserialize the path as hex.")?;

		// Deserialize.
		let module = serde_json::from_slice(&data)
			.map_err(Error::other)
			.wrap_err("Failed to deserialize the identifier.")?;

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
