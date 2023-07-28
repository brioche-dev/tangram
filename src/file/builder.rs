use super::File;
use crate::{artifact::Artifact, blob::Blob, error::Result, instance::Instance};

impl File {
	#[must_use]
	pub fn builder(contents: Blob) -> Builder {
		Builder::new(contents)
	}
}

pub struct Builder {
	contents: Blob,
	executable: bool,
	references: Vec<Artifact>,
}

impl Builder {
	#[must_use]
	pub fn new(contents: Blob) -> Self {
		Self {
			contents,
			executable: false,
			references: Vec::new(),
		}
	}

	#[must_use]
	pub fn contents(mut self, contents: Blob) -> Self {
		self.contents = contents;
		self
	}

	#[must_use]
	pub fn executable(mut self, executable: bool) -> Self {
		self.executable = executable;
		self
	}

	#[must_use]
	pub fn references(mut self, references: Vec<Artifact>) -> Self {
		self.references = references;
		self
	}

	pub async fn build(self, tg: &Instance) -> Result<File> {
		File::new(tg, &self.contents, self.executable, &self.references).await
	}
}
