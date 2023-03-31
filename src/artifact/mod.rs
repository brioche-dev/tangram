pub use self::{hash::Hash, hash::STRING_LENGTH as HASH_STRING_LENGTH, tracker::Tracker};
use crate::{directory::Directory, file::File, symlink::Symlink};

pub mod add;
mod bundle;
mod get;
mod hash;
mod references;
mod serialize;
pub mod tracker;
mod util;

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
#[serde(tag = "kind", content = "value")]
pub enum Artifact {
	#[buffalo(id = 0)]
	#[serde(rename = "directory")]
	Directory(Directory),

	#[buffalo(id = 1)]
	#[serde(rename = "file")]
	File(File),

	#[buffalo(id = 2)]
	#[serde(rename = "symlink")]
	Symlink(Symlink),
}
