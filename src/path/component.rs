#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Component {
	ParentDir,
	Normal(String),
}

impl Component {
	#[must_use]
	pub fn as_str(&self) -> &str {
		match self {
			Component::ParentDir => "..",
			Component::Normal(name) => name,
		}
	}

	#[must_use]
	pub fn as_normal(&self) -> Option<&str> {
		match self {
			Component::ParentDir => None,
			Component::Normal(name) => Some(name),
		}
	}
}
