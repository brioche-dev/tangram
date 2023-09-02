use crate::error::{return_error, Error, Result};
use crate::subpath::Subpath;
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
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[tangram_serialize(into = "String", try_from = "String")]
#[serde(into = "String", try_from = "String")]
pub struct Relpath {
	/// The number of leading parent components.
	pub(crate) parents: usize,

	/// The subpath.
	pub(crate) subpath: Subpath,
}

crate::value!(Relpath);

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

impl std::fmt::Display for Relpath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for _ in 0..self.parents {
			write!(f, "../")?;
		}
		write!(f, "{}", self.subpath)?;
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

impl From<Relpath> for String {
	fn from(value: Relpath) -> Self {
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

impl From<Subpath> for Relpath {
	fn from(value: Subpath) -> Relpath {
		Relpath {
			parents: 0,
			subpath: value,
		}
	}
}

impl From<Relpath> for PathBuf {
	fn from(value: Relpath) -> Self {
		value.to_string().into()
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
