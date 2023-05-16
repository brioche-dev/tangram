use crate::util::fs;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Mount {
	pub kind: Kind,
	pub mode: Mode,
	pub host_path: fs::PathBuf,
	pub guest_path: fs::PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
	File,
	Directory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
	ReadOnly,
	ReadWrite,
}
