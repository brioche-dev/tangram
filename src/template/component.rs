use crate as tg;

#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
pub enum Component {
	#[tangram_serialize(id = 0)]
	String(String),
	#[tangram_serialize(id = 1)]
	Artifact(tg::Artifact),
	#[tangram_serialize(id = 2)]
	Placeholder(tg::Placeholder),
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
	pub fn as_artifact(&self) -> Option<&tg::Artifact> {
		if let Self::Artifact(artifact) = self {
			Some(artifact)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_placeholder(&self) -> Option<&tg::Placeholder> {
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
	pub fn into_artifact(self) -> Option<tg::Artifact> {
		if let Self::Artifact(artifact) = self {
			Some(artifact)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_placeholder(self) -> Option<tg::Placeholder> {
		if let Self::Placeholder(placeholder) = self {
			Some(placeholder)
		} else {
			None
		}
	}
}
