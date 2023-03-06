pub use self::component::Component;
use crate::os;
use anyhow::bail;
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

	pub fn parent(&mut self) {
		self.push(Component::ParentDir);
	}

	pub fn join(&mut self, other: Path) {
		for component in other.components {
			self.push(component);
		}
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

		match self.components.as_slice() {
			// If the path is empty, then write ".".
			[] => {
				write!(f, ".")?;
			},

			// If the path starts with a normal component, then write "./" before the path.
			[Component::Normal(_), ..] => {
				write!(f, "./{string}")?;
			},

			// If the path starts with a parent dir component, then just write the path.
			[Component::ParentDir, ..] => {
				write!(f, "{string}")?;
			},
		}
		Ok(())
	}
}

impl std::str::FromStr for Path {
	type Err = anyhow::Error;

	fn from_str(string: &str) -> Result<Self, Self::Err> {
		// Absolute paths are not allowed.
		if string.starts_with('/') {
			bail!("Absolute paths are not allowed.");
		}

		// Create the path.
		let mut path = Path {
			components: Vec::new(),
		};

		// Split the string by the path separator and handle each component.
		for string in string.split('/') {
			match string {
				"" => {
					bail!("Empty path components are not allowed.");
				},

				// Ignore current dir components.
				"." => {},

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
	type Error = anyhow::Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
	}
}

impl From<Path> for os::PathBuf {
	fn from(value: Path) -> Self {
		value.to_string().into()
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
