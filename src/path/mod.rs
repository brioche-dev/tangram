pub use self::component::Component;
use crate::path;
use itertools::Itertools;

pub mod component;

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
		Path::default()
	}

	#[must_use]
	pub fn parent(&self) -> Path {
		let mut components = self.components.clone();
		components.push(Component::ParentDir);
		Path { components }
	}

	pub fn push(&mut self, component: Component) {
		self.components.push(component);
	}

	#[must_use]
	pub fn join(&self, other: &Path) -> Path {
		let components = self
			.components
			.iter()
			.chain(other.components.iter())
			.cloned()
			.collect();
		Path { components }
	}

	#[must_use]
	pub fn normalize(&self) -> Path {
		let mut normalized_path = Path::new();

		for component in &self.components {
			match component {
				Component::CurrentDir => {
					// Skip current dir components.
				},

				Component::ParentDir => {
					if normalized_path
						.components
						.iter()
						.all(|component| matches!(component, Component::ParentDir))
					{
						// If the normalized path is zero or more parent dir components, then add a parent dir component.
						normalized_path.push(Component::ParentDir);
					} else {
						// Otherwise, remove the last component.
						normalized_path.components.pop();
					}
				},

				Component::Normal(name) => {
					// Add the component.
					normalized_path.push(Component::Normal(name.clone()));
				},
			}
		}

		normalized_path
	}

	#[must_use]
	pub fn file_name(&self) -> Option<&str> {
		self.components.last()?.as_normal()
	}

	#[must_use]
	pub fn extension(&self) -> Option<&str> {
		self.components.last()?.as_normal()?.split('.').last()
	}
}

impl From<&str> for Path {
	fn from(value: &str) -> Self {
		let components = value
			.split('/')
			.map(|component| match component {
				"." => Component::CurrentDir,
				".." => Component::ParentDir,
				component => Component::Normal(component.to_owned()),
			})
			.collect();
		Path { components }
	}
}

impl From<String> for Path {
	fn from(value: String) -> Self {
		value.as_str().into()
	}
}

impl From<Path> for String {
	fn from(value: Path) -> Self {
		value.to_string()
	}
}

impl std::fmt::Display for Path {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let string = self
			.components
			.iter()
			.map(path::component::Component::as_str)
			.join("/");
		write!(f, "{string}")?;
		Ok(())
	}
}

impl FromIterator<Component> for Path {
	fn from_iter<T: IntoIterator<Item = Component>>(iter: T) -> Self {
		let components = iter.into_iter().collect();
		Path { components }
	}
}
