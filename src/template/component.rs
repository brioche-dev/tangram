use crate::{artifact, placeholder};

#[derive(Clone, Debug)]
pub enum Value {
	String(String),
	Artifact(artifact::Handle),
	Placeholder(placeholder::Value),
}

#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
pub enum Data {
	#[tangram_serialize(id = 0)]
	String(crate::string::Data),
	#[tangram_serialize(id = 1)]
	Artifact(artifact::Id),
	#[tangram_serialize(id = 2)]
	Placeholder(crate::placeholder::Data),
}

impl Value {
	#[must_use]
	pub fn as_string(&self) -> Option<&str> {
		if let Self::String(string) = self {
			Some(string)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_artifact(&self) -> Option<&artifact::Handle> {
		if let Self::Artifact(artifact) = self {
			Some(artifact)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_placeholder(&self) -> Option<&placeholder::Value> {
		if let Self::Placeholder(placeholder) = self {
			Some(placeholder)
		} else {
			None
		}
	}
}

impl Value {
	#[must_use]
	pub fn into_string(self) -> Option<String> {
		if let Self::String(string) = self {
			Some(string)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_artifact(self) -> Option<artifact::Handle> {
		if let Self::Artifact(artifact) = self {
			Some(artifact)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_placeholder(self) -> Option<placeholder::Value> {
		if let Self::Placeholder(placeholder) = self {
			Some(placeholder)
		} else {
			None
		}
	}
}
