use crate::{artifact::Artifact, placeholder::Placeholder};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Component {
	String(String),
	Artifact(Artifact),
	Placeholder(Placeholder),
}

impl Component {
	#[must_use]
	pub fn as_string(&self) -> Option<&str> {
		if let Self::String(string) = self {
			Some(string)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_artifact(&self) -> Option<&Artifact> {
		if let Self::Artifact(artifact) = self {
			Some(artifact)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_placeholder(&self) -> Option<&Placeholder> {
		if let Self::Placeholder(placeholder) = self {
			Some(placeholder)
		} else {
			None
		}
	}
}

impl Component {
	#[must_use]
	pub fn into_string(self) -> Option<String> {
		if let Self::String(string) = self {
			Some(string)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_artifact(self) -> Option<Artifact> {
		if let Self::Artifact(artifact) = self {
			Some(artifact)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_placeholder(self) -> Option<Placeholder> {
		if let Self::Placeholder(placeholder) = self {
			Some(placeholder)
		} else {
			None
		}
	}
}

impl std::fmt::Display for Component {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Component::String(value) => f.write_str(value),
			Component::Artifact(value) => f.write_str(&format!("{value}")),
			Component::Placeholder(value) => f.write_str(&value.name.to_string()),
		}
	}
}
