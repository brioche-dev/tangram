use super::{Artifact, Dependency, Directory, File, Symlink};

impl Artifact {
	#[must_use]
	pub fn as_directory(&self) -> Option<&Directory> {
		if let Artifact::Directory(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_file(&self) -> Option<&File> {
		if let Artifact::File(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_symlink(&self) -> Option<&Symlink> {
		if let Artifact::Symlink(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_dependency(&self) -> Option<&Dependency> {
		if let Artifact::Dependency(v) = self {
			Some(v)
		} else {
			None
		}
	}
}

impl Artifact {
	#[must_use]
	pub fn into_directory(self) -> Option<Directory> {
		if let Artifact::Directory(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_file(self) -> Option<File> {
		if let Artifact::File(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_symlink(self) -> Option<Symlink> {
		if let Artifact::Symlink(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_dependency(self) -> Option<Dependency> {
		if let Artifact::Dependency(v) = self {
			Some(v)
		} else {
			None
		}
	}
}
