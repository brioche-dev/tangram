use crate::object::ObjectHash;
use derive_more::{Display, FromStr};

#[derive(Clone, Debug, Display, Eq, FromStr, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(from = "ArtifactSerde", into = "ArtifactSerde")]
pub struct Artifact {
	pub(super) object_hash: ObjectHash,
}

impl Artifact {
	#[must_use]
	pub fn object_hash(&self) -> ObjectHash {
		self.object_hash
	}
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "_tangram")]
enum ArtifactSerde {
	#[serde(rename = "artifact")]
	Artifact { hash: ObjectHash },
}

impl From<Artifact> for ArtifactSerde {
	fn from(value: Artifact) -> ArtifactSerde {
		ArtifactSerde::Artifact {
			hash: value.object_hash,
		}
	}
}

impl From<ArtifactSerde> for Artifact {
	fn from(value: ArtifactSerde) -> Self {
		let ArtifactSerde::Artifact { hash } = value;
		Artifact { object_hash: hash }
	}
}
