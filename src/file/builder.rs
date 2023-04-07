use super::File;
use crate::{artifact::Artifact, blob::Blob, error::Result, instance::Instance};

impl File {
	#[must_use]
	pub fn builder(blob: Blob) -> Builder {
		Builder::new(blob)
	}
}

pub struct Builder {
	blob: Blob,
	executable: bool,
	references: Vec<Artifact>,
}

impl Builder {
	#[must_use]
	pub fn new(blob: Blob) -> Self {
		Self {
			blob,
			executable: false,
			references: Vec::new(),
		}
	}

	#[must_use]
	pub fn blob(mut self, blob: Blob) -> Self {
		self.blob = blob;
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
		File::new(tg, self.blob, self.executable, &self.references).await
	}
}
