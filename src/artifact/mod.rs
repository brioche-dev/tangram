pub use self::{hash::Hash, tracker::Tracker};
use crate::{directory::Directory, file::File, symlink::Symlink};

pub mod add;
mod get;
mod hash;
mod serialize;
pub mod tracker;
mod util;
mod vendor;

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
