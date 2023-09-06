use crate::{
	error::{error, Error, Result},
	relpath::Relpath,
};
use itertools::Itertools;
use std::path::PathBuf;

crate::id!();

crate::kind!(Subpath);

#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

/// A subpath value.
pub type Value = Subpath;

/// Subpath data.
pub type Data = Subpath;

impl Value {
	#[must_use]
	pub fn from_data(data: Data) -> Self {
		data
	}

	#[must_use]
	pub fn to_data(&self) -> Data {
		self.clone()
	}
}

/// A subpath.
#[derive(
	Clone,
	Debug,
	Default,
	Eq,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[tangram_serialize(into = "String", try_from = "String")]
#[serde(into = "String", try_from = "String")]
pub struct Subpath {
	pub(crate) components: Vec<String>,
}

impl Subpath {
	#[must_use]
	pub fn empty() -> Subpath {
		Subpath { components: vec![] }
	}

	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.components.is_empty()
	}

	#[must_use]
	pub fn components(&self) -> &[String] {
		&self.components
	}

	#[must_use]
	pub fn join(mut self, other: Self) -> Self {
		self.components.extend(other.components);
		self
	}

	#[must_use]
	pub fn extension(&self) -> Option<&str> {
		self.components
			.last()
			.and_then(|name| name.split('.').last())
	}

	#[must_use]
	pub fn into_relpath(self) -> Relpath {
		self.into()
	}
}

impl std::fmt::Display for Subpath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		#[allow(unstable_name_collisions)]
		for string in self.components.iter().map(AsRef::as_ref).intersperse("/") {
			write!(f, "{string}")?;
		}
		Ok(())
	}
}

impl std::str::FromStr for Subpath {
	type Err = Error;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		let path: Relpath = s.parse()?;
		let path = path.try_into()?;
		Ok(path)
	}
}

impl From<Subpath> for String {
	fn from(value: Subpath) -> Self {
		value.to_string()
	}
}

impl TryFrom<String> for Subpath {
	type Error = Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
	}
}

impl TryFrom<&str> for Subpath {
	type Error = Error;

	fn try_from(value: &str) -> Result<Self, Self::Error> {
		value.parse()
	}
}

impl TryFrom<Relpath> for Subpath {
	type Error = Error;

	fn try_from(value: Relpath) -> Result<Subpath, Self::Error> {
		if value.parents() == 0 {
			Ok(value.subpath)
		} else {
			Err(error!(r#"The number of parents is not zero."#))
		}
	}
}

impl From<Subpath> for PathBuf {
	fn from(value: Subpath) -> Self {
		value.to_string().into()
	}
}

impl FromIterator<String> for Subpath {
	fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
		Subpath {
			components: iter.into_iter().collect(),
		}
	}
}
