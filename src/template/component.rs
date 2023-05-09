use crate::{artifact::Artifact, placeholder::Placeholder};

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind", content = "value")]
pub enum Component {
	#[serde(rename = "string")]
	String(String),

	#[serde(rename = "artifact")]
	Artifact(Artifact),

	#[serde(rename = "placeholder")]
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
