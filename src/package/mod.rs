pub use self::hash::PackageHash;
use crate::artifact::ArtifactHash;
use std::collections::BTreeMap;

mod add;
mod checkin;
mod get;
mod hash;
mod lockfile;
mod serialize;

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
pub struct Package {
	#[buffalo(id = 0)]
	pub source: ArtifactHash,

	#[buffalo(id = 1)]
	pub dependencies: BTreeMap<String, PackageHash>,
}
