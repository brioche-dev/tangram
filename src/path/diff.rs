use super::{Component, Path};
use crate::{error::Result, return_error};

impl Path {
	#[must_use]
	pub fn diff(&self, path: &Path) -> Path {
		self.try_diff(path).unwrap()
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
