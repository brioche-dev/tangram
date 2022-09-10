use crate::object;
use derive_more::{Display, FromStr};

#[derive(
	Clone, Copy, Debug, Display, Eq, FromStr, Hash, PartialEq, serde::Deserialize, serde::Serialize,
)]
#[serde(from = "ArtifactSerde", into = "ArtifactSerde")]
pub struct Artifact {
	object_hash: object::Hash,
}

impl Artifact {
	#[must_use]
	pub fn new(object_hash: object::Hash) -> Artifact {
		Artifact { object_hash }
	}

	#[must_use]
	pub fn object_hash(&self) -> object::Hash {
		self.object_hash
	}
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "_tangram")]
enum ArtifactSerde {
	#[serde(rename = "artifact")]
	Artifact { object_hash: object::Hash },
}

impl From<Artifact> for ArtifactSerde {
	fn from(value: Artifact) -> ArtifactSerde {
		ArtifactSerde::Artifact {
			object_hash: value.object_hash,
		}
	}
}

impl From<ArtifactSerde> for Artifact {
	fn from(value: ArtifactSerde) -> Self {
		let ArtifactSerde::Artifact { object_hash } = value;
		Artifact { object_hash }
	}
}
