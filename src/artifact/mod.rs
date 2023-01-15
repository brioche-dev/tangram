pub use self::add::AddArtifactOutcome;
pub use self::{
	dependency::Dependency, directory::Directory, file::File, hash::ArtifactHash, symlink::Symlink,
};

mod add;
mod dependency;
mod directory;
mod file;
mod get;
mod hash;
mod serialize;
mod symlink;
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
#[serde(tag = "type", content = "value")]
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

	#[buffalo(id = 3)]
	#[serde(rename = "dependency")]
	Dependency(Dependency),
}
