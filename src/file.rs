use crate::{artifact, blob, Client, Result};

crate::id!();

crate::kind!(File);

/// A file handle.
#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

/// A file value.
#[derive(Clone, Debug)]
pub struct Value {
	/// The file's contents.
	pub contents: blob::Handle,

	/// Whether the file is executable.
	pub executable: bool,

	/// The file's references.
	pub references: Vec<artifact::Handle>,
}

/// File data.
#[derive(
	Clone,
	Debug,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
pub struct Data {
	/// The file's contents.
	#[tangram_serialize(id = 0)]
	pub contents: blob::Id,

	/// Whether the file is executable.
	#[tangram_serialize(id = 1)]
	pub executable: bool,

	/// The file's references.
	#[tangram_serialize(id = 2)]
	pub references: Vec<artifact::Id>,
}

impl Handle {
	#[must_use]
	pub fn new(
		contents: blob::Handle,
		executable: bool,
		references: Vec<artifact::Handle>,
	) -> Self {
		Self::with_value(Value {
			contents,
			executable,
			references,
		})
	}

	#[must_use]
	pub fn builder(contents: blob::Handle) -> Builder {
		Builder::new(contents)
	}

	pub async fn contents(&self, tg: &Client) -> Result<blob::Handle> {
		Ok(self.value(tg).await?.contents.clone())
	}

	pub async fn executable(&self, tg: &Client) -> Result<bool> {
		Ok(self.value(tg).await?.executable)
	}

	pub async fn references(&self, tg: &Client) -> Result<&[artifact::Handle]> {
		Ok(self.value(tg).await?.references.as_slice())
	}
}

impl Value {
	#[must_use]
	pub fn from_data(data: Data) -> Self {
		let contents = blob::Handle::with_id(data.contents);
		let executable = data.executable;
		let references = data
			.references
			.into_iter()
			.map(artifact::Handle::with_id)
			.collect();
		Self {
			contents,
			executable,
			references,
		}
	}

	#[must_use]
	pub fn to_data(&self) -> Data {
		let contents = self.contents.expect_id();
		let executable = self.executable;
		let references = self
			.references
			.iter()
			.map(artifact::Handle::expect_id)
			.collect();
		Data {
			contents,
			executable,
			references,
		}
	}

	#[must_use]
	pub fn children(&self) -> Vec<crate::Handle> {
		let contents = self.contents.clone().into();
		let references = self
			.references
			.iter()
			.map(|reference| reference.clone().into());
		std::iter::once(contents).chain(references).collect()
	}
}

impl Data {
	#[must_use]
	pub fn children(&self) -> Vec<crate::Id> {
		std::iter::once(self.contents.into())
			.chain(self.references.iter().copied().map(Into::into))
			.collect()
	}
}

pub struct Builder {
	contents: blob::Handle,
	executable: bool,
	references: Vec<artifact::Handle>,
}

impl Builder {
	#[must_use]
	pub fn new(contents: blob::Handle) -> Self {
		Self {
			contents,
			executable: false,
			references: Vec::new(),
		}
	}

	#[must_use]
	pub fn contents(mut self, contents: blob::Handle) -> Self {
		self.contents = contents;
		self
	}

	#[must_use]
	pub fn executable(mut self, executable: bool) -> Self {
		self.executable = executable;
		self
	}

	#[must_use]
	pub fn references(mut self, references: Vec<artifact::Handle>) -> Self {
		self.references = references;
		self
	}

	#[must_use]
	pub fn build(self) -> Handle {
		Handle::new(self.contents, self.executable, self.references)
	}
}
