use crate::error::{error, return_error, Error, Result};
use itertools::Itertools;
use std::path::PathBuf;

/// A relative path.
#[derive(
	Clone,
	Debug,
	Default,
	Eq,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	buffalo::Serialize,
	buffalo::Deserialize,
	serde::Serialize,
	serde::Deserialize,
)]
#[buffalo(into = "String", try_from = "String")]
#[serde(into = "String", try_from = "String")]
pub struct Relpath {
	/// The number of leading parent components.
	parents: usize,

	/// The subpath.
	subpath: Subpath,
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
	buffalo::Serialize,
	buffalo::Deserialize,
	serde::Serialize,
	serde::Deserialize,
)]
#[buffalo(into = "String", try_from = "String")]
#[serde(into = "String", try_from = "String")]
pub struct Subpath {
	components: Vec<String>,
}

impl Relpath {
	#[must_use]
	pub fn empty() -> Relpath {
		Relpath {
			parents: 0,
			subpath: Subpath::empty(),
		}
	}

	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.parents == 0 && self.subpath.is_empty()
	}

	#[must_use]
	pub fn parents(&self) -> usize {
		self.parents
	}

	#[must_use]
	pub fn subpath(&self) -> &Subpath {
		&self.subpath
	}

	#[must_use]
	pub fn parent(mut self) -> Self {
		if self.subpath.is_empty() {
			self.parents += 1;
		} else {
			self.subpath.components.pop();
		}
		self
	}

	#[must_use]
	pub fn join(mut self, other: Relpath) -> Self {
		for _ in 0..other.parents {
			self = self.parent();
		}
		self.subpath.components.extend(other.subpath.components);
		self
	}

	#[must_use]
	pub fn extension(&self) -> Option<&str> {
		self.subpath.extension()
	}

	#[must_use]
	pub fn try_into_subpath(self) -> Option<Subpath> {
		self.try_into().ok()
	}
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

impl std::fmt::Display for Relpath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for _ in 0..self.parents {
			write!(f, "../")?;
		}
		write!(f, "{}", self.subpath)?;
		Ok(())
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

impl std::str::FromStr for Relpath {
	type Err = Error;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		if s.starts_with('/') {
			return_error!(r#"A relpath cannot start with a path separator."#);
		}
		let mut path = Self::empty();
		for component in s.split('/') {
			match component {
				"" | "." => {},
				".." => {
					path = path.parent();
				},
				_ => {
					path.subpath.components.push(component.to_owned());
				},
			}
		}
		Ok(path)
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

impl From<Relpath> for String {
	fn from(value: Relpath) -> Self {
		value.to_string()
	}
}

impl From<Subpath> for String {
	fn from(value: Subpath) -> Self {
		value.to_string()
	}
}

impl TryFrom<String> for Relpath {
	type Error = Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
	}
}

impl TryFrom<&str> for Relpath {
	type Error = Error;

	fn try_from(value: &str) -> Result<Self, Self::Error> {
		value.parse()
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

impl From<Subpath> for Relpath {
	fn from(value: Subpath) -> Relpath {
		Relpath {
			parents: 0,
			subpath: value,
		}
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

impl From<Relpath> for PathBuf {
	fn from(value: Relpath) -> Self {
		value.to_string().into()
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

impl Relpath {
	#[must_use]
	pub fn diff(&self, src: &Relpath) -> Relpath {
		self.try_diff(src).unwrap()
	}

	pub fn try_diff(&self, src: &Relpath) -> Result<Relpath> {
		let src = src;
		let dst = self;

		// Remove the common parents.
		let common_parents = std::cmp::min(src.parents, dst.parents);

		// If the src parents is greater than the common parents, then return an error.
		if src.parents > common_parents {
			return_error!(r#"Invalid comparison."#);
		}

		// If the src and dst have the same number of parents, then remove the common subpath components.
		let common_components = if src.parents == dst.parents {
			std::iter::zip(&src.subpath.components, &dst.subpath.components)
				.take_while(|(a, b)| a == b)
				.count()
		} else {
			0
		};

		// Create the path.
		let parents =
			src.subpath.components.len() - common_components + dst.parents - common_parents;
		let subpath = Subpath {
			components: dst
				.subpath
				.components
				.iter()
				.skip(common_components)
				.cloned()
				.collect(),
		};
		let path = Relpath { parents, subpath };

		Ok(path)
	}
}

#[cfg(test)]
mod tests {
	use super::Relpath;

	#[test]
	fn test_diff() {
		let src = Relpath::empty();
		let dst = Relpath::empty();
		let left = dst.diff(&src);
		let right = Relpath::empty();
		assert_eq!(left, right);

		let src = Relpath::empty();
		let dst = Relpath::try_from("foo").unwrap();
		let left = dst.diff(&src);
		let right = Relpath::try_from("foo").unwrap();
		assert_eq!(left, right);

		let src = Relpath::empty();
		let dst = Relpath::try_from("foo/bar").unwrap();
		let left = dst.diff(&src);
		let right = Relpath::try_from("foo/bar").unwrap();
		assert_eq!(left, right);

		let src = Relpath::empty();
		let dst = Relpath::try_from("../../foo/bar").unwrap();
		let left = dst.diff(&src);
		let right = Relpath::try_from("../../foo/bar").unwrap();
		assert_eq!(left, right);

		let src = Relpath::try_from("foo").unwrap();
		let dst = Relpath::empty();
		let left = dst.diff(&src);
		let right = Relpath::try_from("..").unwrap();
		assert_eq!(left, right);

		let src = Relpath::try_from("foo/bar").unwrap();
		let dst = Relpath::empty();
		let left = dst.diff(&src);
		let right = Relpath::try_from("../..").unwrap();
		assert_eq!(left, right);

		let src = Relpath::try_from("foo/bar/baz").unwrap();
		let dst = Relpath::empty();
		let left = dst.diff(&src);
		let right = Relpath::try_from("../../..").unwrap();
		assert_eq!(left, right);

		let src = Relpath::try_from("foo").unwrap();
		let dst = Relpath::try_from("foo").unwrap();
		let left = dst.diff(&src);
		let right = Relpath::empty();
		assert_eq!(left, right);

		let src = Relpath::try_from("foo/bar").unwrap();
		let dst = Relpath::try_from("foo/bar").unwrap();
		let left = dst.diff(&src);
		let right = Relpath::empty();
		assert_eq!(left, right);

		let src = Relpath::try_from("../../foo/bar").unwrap();
		let dst = Relpath::try_from("../../foo/bar").unwrap();
		let left = dst.diff(&src);
		let right = Relpath::empty();
		assert_eq!(left, right);

		let src = Relpath::try_from("foo").unwrap();
		let dst = Relpath::try_from("foo/bar").unwrap();
		let left = dst.diff(&src);
		let right = Relpath::try_from("bar").unwrap();
		assert_eq!(left, right);

		let src = Relpath::try_from("foo/baz/buzz").unwrap();
		let dst = Relpath::try_from("foo/bar/baz").unwrap();
		let left = dst.diff(&src);
		let right = Relpath::try_from("../../bar/baz").unwrap();
		assert_eq!(left, right);

		let src = Relpath::try_from("../foo/bar").unwrap();
		let dst = Relpath::try_from("../foo").unwrap();
		let left = dst.diff(&src);
		let right = Relpath::try_from("..").unwrap();
		assert_eq!(left, right);

		let src = Relpath::try_from("../foo/fizz/buzz").unwrap();
		let dst = Relpath::try_from("../foo/bar/baz").unwrap();
		let left = dst.diff(&src);
		let right = Relpath::try_from("../../bar/baz").unwrap();
		assert_eq!(left, right);

		let src = Relpath::try_from("../foo/bar").unwrap();
		let dst = Relpath::try_from("..").unwrap();
		let left = dst.diff(&src);
		let right = Relpath::try_from("../..").unwrap();
		assert_eq!(left, right);

		let src = Relpath::try_from("../foo").unwrap();
		let dst = Relpath::empty();
		assert!(dst.try_diff(&src).is_err());

		let src = Relpath::try_from("../../bar").unwrap();
		let dst = Relpath::try_from("../foo").unwrap();
		assert!(dst.try_diff(&src).is_err());

		let src = Relpath::try_from("../../../bar").unwrap();
		let dst = Relpath::try_from("../../foo").unwrap();
		assert!(dst.try_diff(&src).is_err());

		let src = Relpath::try_from("foo/bar/baz").unwrap();
		let dst = Relpath::try_from("../../").unwrap();
		let left = dst.diff(&src);
		let right = Relpath::try_from("../../../../../").unwrap();
		assert_eq!(left, right);
	}
}
