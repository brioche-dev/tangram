#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Component {
	Parent,
	Normal(String),
}

impl Component {
	#[must_use]
	pub fn is_parent(&self) -> bool {
		matches!(self, Component::Parent)
	}

	#[must_use]
	pub fn is_normal(&self) -> bool {
		matches!(self, Component::Normal(_))
	}

	#[must_use]
	pub fn as_normal(&self) -> Option<&str> {
		match self {
			Component::Parent => None,
			Component::Normal(name) => Some(name),
		}
	}

	#[must_use]
	pub fn as_str(&self) -> &str {
		match self {
			Component::Parent => "..",
			Component::Normal(name) => name,
		}
	}
}
