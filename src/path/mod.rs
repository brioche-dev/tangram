pub use self::component::Component;
use crate::{
	error::{Error, Result},
	return_error,
	util::fs,
};
use itertools::Itertools;

pub mod component;

/// A relative path that is always normalized.
#[derive(
	Clone,
	Debug,
	Default,
	Eq,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	serde::Serialize,
	serde::Deserialize,
	buffalo::Serialize,
	buffalo::Deserialize,
)]
#[serde(into = "String", try_from = "String")]
#[buffalo(into = "String", try_from = "String")]
pub struct Path {
	pub components: Vec<Component>,
}

impl Path {
	#[must_use]
	pub fn new() -> Path {
		Path {
			components: Vec::new(),
		}
	}

	pub fn push(&mut self, component: Component) {
		match component {
			Component::ParentDir => {
				if self
					.components
					.last()
					.map_or(true, |component| matches!(component, Component::ParentDir))
				{
					self.components.push(Component::ParentDir);
				} else {
					self.components.pop();
				}
			},
			Component::Normal(_) => {
				self.components.push(component);
			},
		}
	}

	#[must_use]
	pub fn join(mut self, other: impl IntoIterator<Item = Component>) -> Self {
		for component in other {
			self.push(component);
		}

		self
	}

	#[must_use]
	pub fn extension(&self) -> Option<&str> {
		self.components
			.last()
			.and_then(Component::as_normal)
			.and_then(|name| name.split('.').last())
	}

	pub fn try_diff(&self, base: &Path) -> Result<Path> {
		let mut base_components = base.components.clone();
		let mut target_components = self.components.clone();

		// Continually remove compoenents from the front of both paths until they are equal. This will remove the common ancestor of both paths.
		loop {
			let dir_component = base_components.first();
			let target_component = target_components.first();

			match (dir_component, target_component) {
				(Some(dir_component), Some(target_component))
					if dir_component == target_component =>
				{
					base_components.remove(0);
					target_components.remove(0);
				},
				_ => {
					break;
				},
			}
		}

		// Add a parent dir component for each remaining base path component. This will ascend until reaching the common ancestor of the base path and target path.
		let mut path = Path::new();
		for component in &base_components {
			match component {
				Component::ParentDir => {
					return_error!("Path {self} is not a child of the base path {base}.");
				},
				Component::Normal(_) => {
					path.push(Component::ParentDir);
				},
			}
		}

		// Traverse from the common ancestor to the target path.
		for component in target_components {
			path.push(component);
		}

		Ok(path)
	}

	#[must_use]
	pub fn diff(&self, path: &Path) -> Path {
		self.try_diff(path).unwrap()
	}
}

impl std::fmt::Display for Path {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		// Join the components with the path separator.
		let string = self.components.iter().map(Component::as_str).join("/");

		// Write the string.
		write!(f, "{string}")?;

		Ok(())
	}
}

impl std::str::FromStr for Path {
	type Err = Error;

	fn from_str(string: &str) -> Result<Self, Self::Err> {
		// Create the path.
		let mut path = Path {
			components: Vec::new(),
		};

		// Split the string by the path separator.
		let components = string.split('/');

		// Push each component.
		for string in components {
			match string {
				// Ignore empty and current dir components.
				"" | "." => {},

				// Handle parent dir components.
				".." => path.push(Component::ParentDir),

				// Handle normal components.
				string => path.push(Component::Normal(string.to_owned())),
			}
		}

		Ok(path)
	}
}

impl From<Path> for String {
	fn from(value: Path) -> Self {
		value.to_string()
	}
}

impl TryFrom<String> for Path {
	type Error = Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
	}
}

impl From<Path> for fs::PathBuf {
	fn from(value: Path) -> Self {
		value.to_string().into()
	}
}

impl IntoIterator for Path {
	type Item = Component;
	type IntoIter = std::vec::IntoIter<Component>;

	fn into_iter(self) -> Self::IntoIter {
		self.components.into_iter()
	}
}

impl FromIterator<Component> for Path {
	fn from_iter<T: IntoIterator<Item = Component>>(iter: T) -> Self {
		let mut path = Path::new();
		for component in iter {
			path.push(component);
		}
		path
	}
}

#[cfg(test)]
mod tests {
	use super::Path;

	use assert_matches::assert_matches;

	fn path(path: &str) -> Path {
		path.parse().unwrap()
	}

	#[test]
	fn test_diff_paths() {
		// Diffing the empty path to itself should return the empty path.
		assert_eq!(Path::new().diff(&Path::new()), Path::new());

		// Diffing any path with the empty path should return the original path.
		assert_eq!(path("foo").diff(&Path::new()), path("foo"));
		assert_eq!(path("foo/bar").diff(&Path::new()), path("foo/bar"));
		assert_eq!(
			path("../../foo/bar").diff(&Path::new()),
			path("../../foo/bar")
		);

		// Diffing the empty path with any path should return parent dir components.
		assert_eq!(Path::new().diff(&path("foo")), path(".."));
		assert_eq!(Path::new().diff(&path("foo/bar")), path("../.."));
		assert_eq!(Path::new().diff(&path("foo/bar/baz")), path("../../.."));

		// Diffing a path with itself should return the empty path.
		assert_eq!(path("foo").diff(&path("foo")), Path::new());
		assert_eq!(path("foo/bar").diff(&path("foo/bar")), Path::new());
		assert_eq!(
			path("../../foo/bar").diff(&path("../../foo/bar")),
			Path::new()
		);

		// Diffing a path should ascend until reaching the common base dir then descend to the target directory.
		assert_eq!(path("foo/bar").diff(&path("foo")), path("bar"));
		assert_eq!(
			path("foo/bar/baz").diff(&path("foo/baz/buzz")),
			path("../../bar/baz")
		);
		assert_eq!(path("../foo").diff(&path("../foo/bar")), path(".."));
		assert_eq!(
			path("../foo/bar/baz").diff(&path("../foo/fizz/buzz")),
			path("../../bar/baz"),
		);
		assert_eq!(path("..").diff(&path("../foo/bar")), path("../.."));

		// Diffing a path where the base has more parent dir components is an error.
		assert_matches!(Path::new().try_diff(&path("../foo")), Err(_));
		assert_matches!(path("../foo").try_diff(&path("../../bar")), Err(_));
		assert_matches!(path("../../foo").try_diff(&path("../../../bar")), Err(_));
	}
}
