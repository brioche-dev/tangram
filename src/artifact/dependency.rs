use camino::Utf8PathBuf;

use super::ArtifactHash;

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Dependency {
	#[buffalo(id = 0)]
	pub artifact: ArtifactHash,

	#[buffalo(id = 1)]
	pub path: Option<Utf8PathBuf>,
}
