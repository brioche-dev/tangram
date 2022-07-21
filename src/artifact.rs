use crate::{hash::Hash, object::ObjectHash};

#[derive(Clone, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
#[allow(clippy::module_name_repetitions)]
pub struct ArtifactHash(Hash);

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(from = "ArtifactSerde", into = "ArtifactSerde")]
pub struct Artifact(pub ObjectHash);

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "_tangram")]
enum ArtifactSerde {
	#[serde(rename = "artifact")]
	Artifact { hash: ObjectHash },
}

impl From<Artifact> for ArtifactSerde {
	fn from(value: Artifact) -> ArtifactSerde {
		ArtifactSerde::Artifact { hash: value.0 }
	}
}

impl From<ArtifactSerde> for Artifact {
	fn from(value: ArtifactSerde) -> Self {
		let ArtifactSerde::Artifact { hash } = value;
		Artifact(hash)
	}
}

impl std::ops::Deref for ArtifactHash {
	type Target = Hash;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl std::fmt::Display for ArtifactHash {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}
