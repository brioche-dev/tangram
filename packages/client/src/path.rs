use crate::{Error, Result};
use derive_more::{TryUnwrap, Unwrap};
use std::path::PathBuf;
use tangram_error::WrapErr;

/// Any path.
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
)]
#[serde(into = "String", try_from = "String")]
pub struct Path {
	components: Vec<Component>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, TryUnwrap, Unwrap)]
#[try_unwrap(ref)]
#[unwrap(ref)]
pub enum Component {
	Root,
	Current,
	Parent,
	Normal(String),
}

impl Path {
	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.components.is_empty()
	}

	#[must_use]
	pub fn components(&self) -> &[Component] {
		&self.components
	}

	pub fn push(&mut self, component: Component) {
		// Ignore the component if it is a current directory component.
		if component == Component::Current {
			return;
		}

		// If the component is a root component, then clear the path.
		if component == Component::Root {
			self.components.clear();
		}

		// Add the component to the path.
		self.components.push(component);
	}

	#[must_use]
	pub fn parent(self) -> Self {
		self.join(Component::Parent.into())
	}

	#[must_use]
	pub fn join(mut self, other: Self) -> Self {
		for component in other.components {
			self.push(component);
		}
		self
	}

	#[must_use]
	pub fn normalize(self) -> Self {
		let mut path = Self::default();
		for component in self.components {
			if component == Component::Parent
				&& matches!(path.components.last(), Some(Component::Normal(_)))
			{
				path.components.pop();
			} else {
				path.components.push(component);
			}
		}
		path
	}

	#[must_use]
	pub fn is_absolute(&self) -> bool {
		matches!(self.components.first(), Some(Component::Root))
	}

	#[must_use]
	pub fn extension(&self) -> Option<&str> {
		self.components
			.last()
			.and_then(|component| component.try_unwrap_normal_ref().ok())
			.and_then(|name| name.split('.').last())
	}
}

impl std::fmt::Display for Path {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for (i, component) in self.components.iter().enumerate() {
			match component {
				Component::Root => {
					write!(f, "/")?;
				},
				Component::Current => {
					if i != 0 {
						write!(f, "/")?;
					}
					write!(f, ".")?;
				},
				Component::Parent => {
					if i != 0 {
						write!(f, "/")?;
					}
					write!(f, "..")?;
				},
				Component::Normal(name) => {
					if i != 0 {
						write!(f, "/")?;
					}
					write!(f, "{name}")?;
				},
			}
		}
		Ok(())
	}
}

impl std::str::FromStr for Path {
	type Err = Error;

	fn from_str(mut s: &str) -> std::result::Result<Self, Self::Err> {
		let mut path = Self::default();
		if s.starts_with('/') {
			path.components.push(Component::Root);
			s = &s[1..];
		}
		for component in s.split('/') {
			match component {
				"" | "." => (),
				".." => {
					path.components.push(Component::Parent);
				},
				_ => {
					let component = Component::Normal(component.to_owned());
					path.components.push(component);
				},
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

impl From<Component> for Path {
	fn from(value: Component) -> Self {
		Self {
			components: vec![value],
		}
	}
}

impl FromIterator<Component> for Path {
	fn from_iter<T: IntoIterator<Item = Component>>(iter: T) -> Self {
		Self {
			components: iter.into_iter().collect(),
		}
	}
}

impl From<Path> for PathBuf {
	fn from(value: Path) -> Self {
		value.to_string().into()
	}
}

impl TryFrom<PathBuf> for Path {
	type Error = Error;

	fn try_from(value: PathBuf) -> std::prelude::v1::Result<Self, Self::Error> {
		value
			.as_os_str()
			.to_str()
			.wrap_err("The path must be valid UTF-8.")?
			.parse()
	}
}
