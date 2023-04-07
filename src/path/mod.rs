pub use self::component::Component;
use crate::util::fs;
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
	buffalo::Serialize,
	buffalo::Deserialize,
	serde::Serialize,
	serde::Deserialize,
)]
#[buffalo(into = "String", try_from = "String")]
#[serde(into = "String", try_from = "String")]
pub struct Path {
	components: Vec<Component>,
}

impl Path {
	#[must_use]
	pub fn empty() -> Path {
		Path { components: vec![] }
	}

	pub fn new(path: impl Into<Self>) -> Self {
		path.into()
	}

	#[must_use]
	pub fn components(&self) -> &[Component] {
		&self.components
	}

	pub fn push(&mut self, component: Component) {
		match component {
			Component::Parent => {
				if self
					.components
					.last()
					.map_or(true, |component| matches!(component, Component::Parent))
				{
					self.components.push(Component::Parent);
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
	pub fn join(mut self, other: impl Into<Self>) -> Self {
		for component in other.into() {
			self.push(component);
		}
		self
	}

	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.components.is_empty()
	}

	#[must_use]
	pub fn has_parent_components(&self) -> bool {
		self.components()
			.first()
			.map_or(false, component::Component::is_parent)
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

impl From<&str> for Path {
	fn from(value: &str) -> Self {
		// Create the path.
		let mut path = Path {
			components: Vec::new(),
		};

		// Split the string by the path separator.
		let components = value.split('/');

		// Push each component.
		for string in components {
			match string {
				// Ignore empty and current dir components.
				"" | "." => {},

				// Handle parent dir components.
				".." => path.push(Component::Parent),

				// Handle normal components.
				string => path.push(Component::Normal(string.to_owned())),
			}
		}

		path
	}
}

impl From<Path> for String {
	fn from(value: Path) -> Self {
		value.to_string()
	}
}

impl From<String> for Path {
	fn from(value: String) -> Self {
		value.as_str().into()
	}
}

impl From<&String> for Path {
	fn from(value: &String) -> Self {
		value.as_str().into()
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
		let mut path = Path::empty();
		for component in iter {
			path.push(component);
		}
		path
	}
}

impl From<Component> for Path {
	fn from(value: Component) -> Self {
		Path {
			components: vec![value],
		}
	}
}

impl From<Path> for fs::PathBuf {
	fn from(value: Path) -> Self {
		value.to_string().into()
	}
}
