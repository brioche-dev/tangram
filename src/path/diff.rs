use super::{Component, Path};
use crate::{error::Result, return_error};

impl Path {
	#[must_use]
	pub fn diff(&self, src: &Path) -> Path {
		self.try_diff(src).unwrap()
	}

	pub fn try_diff(&self, src: &Path) -> Result<Path> {
		let mut src = src.clone();
		let mut dst = self.clone();

		// Remove the paths' common ancestor.
		loop {
			match (src.components.first(), dst.components.first()) {
				(Some(src_component), Some(dst_component)) if src_component == dst_component => {
					src.components.remove(0);
					dst.components.remove(0);
				},
				_ => {
					break;
				},
			}
		}

		// If there is no valid path from the source path to the destination path, then return an error.
		if let Some(Component::Parent) = src.components.first() {
			return_error!(r#"There is no valid path from "{src}" to "{dst}"."#);
		}

		// Construct the path.
		let path: Path = std::iter::repeat(Component::Parent)
			.take(src.components.len())
			.collect();
		let path = path.join(dst);

		Ok(path)
	}
}

#[cfg(test)]
mod tests {
	use super::Path;

	use assert_matches::assert_matches;

	#[test]
	fn test() {
		// Diffing the empty path to itself should return the empty path.
		assert_eq!(Path::empty().diff(&Path::empty()), Path::empty());

		// Diffing any path with the empty path should return the original path.
		assert_eq!(Path::new("foo").diff(&Path::empty()), Path::new("foo"));
		assert_eq!(
			Path::new("foo/bar").diff(&Path::empty()),
			Path::new("foo/bar")
		);
		assert_eq!(
			Path::new("../../foo/bar").diff(&Path::empty()),
			Path::new("../../foo/bar")
		);

		// Diffing the empty path with any path should return parent dir components.
		assert_eq!(Path::empty().diff(&Path::new("foo")), Path::new(".."));
		assert_eq!(
			Path::empty().diff(&Path::new("foo/bar")),
			Path::new("../..")
		);
		assert_eq!(
			Path::empty().diff(&Path::new("foo/bar/baz")),
			Path::new("../../..")
		);

		// Diffing a path with itself should return the empty path.
		assert_eq!(Path::new("foo").diff(&Path::new("foo")), Path::empty());
		assert_eq!(
			Path::new("foo/bar").diff(&Path::new("foo/bar")),
			Path::empty()
		);
		assert_eq!(
			Path::new("../../foo/bar").diff(&Path::new("../../foo/bar")),
			Path::empty()
		);

		// Diffing a path should ascend until reaching the common base path, then descend to the target directory.
		assert_eq!(
			Path::new("foo/bar").diff(&Path::new("foo")),
			Path::new("bar")
		);
		assert_eq!(
			Path::new("foo/bar/baz").diff(&Path::new("foo/baz/buzz")),
			Path::new("../../bar/baz")
		);
		assert_eq!(
			Path::new("../foo").diff(&Path::new("../foo/bar")),
			Path::new("..")
		);
		assert_eq!(
			Path::new("../foo/bar/baz").diff(&Path::new("../foo/fizz/buzz")),
			Path::new("../../bar/baz"),
		);
		assert_eq!(
			Path::new("..").diff(&Path::new("../foo/bar")),
			Path::new("../..")
		);

		// Diffing a path where the base has more parent dir components is an error.
		assert_matches!(Path::empty().try_diff(&Path::new("../foo")), Err(_));
		assert_matches!(
			Path::new("../foo").try_diff(&Path::new("../../bar")),
			Err(_)
		);
		assert_matches!(
			Path::new("../../foo").try_diff(&Path::new("../../../bar")),
			Err(_)
		);
	}
}
