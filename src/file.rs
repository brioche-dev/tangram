use crate::{self as tg, error::Result, instance::Instance};

#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
pub struct File {
	/// The file's contents.
	#[tangram_serialize(id = 0)]
	pub contents: tg::Blob,

	/// Whether the file is executable.
	#[tangram_serialize(id = 1)]
	pub executable: bool,

	/// The file's references.
	#[tangram_serialize(id = 2)]
	pub references: Vec<tg::Artifact>,
}

crate::value!(File);

impl tg::File {
	#[must_use]
	pub fn new(contents: tg::Blob, executable: bool, references: Vec<tg::Artifact>) -> Self {
		File {
			contents,
			executable,
			references,
		}
		.into()
	}

	#[must_use]
	pub fn builder(contents: tg::Blob) -> Builder {
		Builder::new(contents)
	}

	pub async fn contents(&self, tg: &Instance) -> Result<tg::Blob> {
		Ok(self.get(tg).await?.contents.clone())
	}

	pub async fn executable(&self, tg: &Instance) -> Result<bool> {
		Ok(self.get(tg).await?.executable)
	}

	pub async fn references(&self, tg: &Instance) -> Result<&[tg::Artifact]> {
		Ok(self.get(tg).await?.references.as_slice())
	}
}

impl File {
	#[must_use]
	pub fn children(&self) -> Vec<tg::Value> {
		let contents = self.contents.clone().into();
		let references = self
			.references
			.iter()
			.map(|reference| reference.clone().into());
		std::iter::once(contents).chain(references).collect()
	}
}

pub struct Builder {
	contents: tg::Blob,
	executable: bool,
	references: Vec<tg::Artifact>,
}

impl Builder {
	#[must_use]
	pub fn new(contents: tg::Blob) -> Self {
		Self {
			contents,
			executable: false,
			references: Vec::new(),
		}
	}

	#[must_use]
	pub fn contents(mut self, contents: tg::Blob) -> Self {
		self.contents = contents;
		self
	}

	#[must_use]
	pub fn executable(mut self, executable: bool) -> Self {
		self.executable = executable;
		self
	}

	#[must_use]
	pub fn references(mut self, references: Vec<tg::Artifact>) -> Self {
		self.references = references;
		self
	}

	pub fn build(self) -> tg::File {
		tg::File::new(self.contents, self.executable, self.references)
	}
}
