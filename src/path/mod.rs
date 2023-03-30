pub use self::component::Component;
use crate::{
	error::{Error, Result},
	util::fs,
};
use itertools::Itertools;

mod component;
mod diff;

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
