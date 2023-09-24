use crate::{artifact, blob, id, object, Artifact, Blob, Client, Result};

#[derive(Clone, Debug)]
pub struct File(Handle);

crate::object!(File);

/// A file value.
#[derive(Clone, Debug)]
pub(crate) struct Object {
	/// The file's contents.
	pub contents: Blob,

	/// Whether the file is executable.
	pub executable: bool,

	/// The file's references.
	pub references: Vec<Artifact>,
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
pub(crate) struct Data {
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

impl File {
	#[must_use]
	pub fn handle(&self) -> &Handle {
		&self.0
	}

	#[must_use]
	pub fn with_id(id: Id) -> Self {
		Self(Handle::with_id(id))
	}

	#[must_use]
	pub fn new(contents: Blob, executable: bool, references: Vec<Artifact>) -> Self {
		Self(Handle::with_object(Object {
			contents,
			executable,
			references,
		}))
	}

	#[must_use]
	pub fn builder(contents: Blob) -> Builder {
		Builder::new(contents)
	}

	pub async fn contents(&self, client: &Client) -> Result<&Blob> {
		Ok(&self.0.object(client).await?.contents)
	}

	pub async fn executable(&self, client: &Client) -> Result<bool> {
		Ok(self.0.object(client).await?.executable)
	}

	pub async fn references(&self, client: &Client) -> Result<&[Artifact]> {
		Ok(self.0.object(client).await?.references.as_slice())
	}
}

impl Id {
	#[must_use]
	pub fn with_data_bytes(bytes: &[u8]) -> Self {
		Self(crate::Id::new_hashed(id::Kind::File, bytes))
	}
}

impl Object {
	#[must_use]
	pub(crate) fn to_data(&self) -> Data {
		let contents = self.contents.handle().expect_id();
		let executable = self.executable;
		let references = self.references.iter().map(Artifact::expect_id).collect();
		Data {
			contents,
			executable,
			references,
		}
	}

	#[must_use]
	pub(crate) fn from_data(data: Data) -> Self {
		let contents = Blob::with_id(data.contents);
		let executable = data.executable;
		let references = data.references.into_iter().map(Artifact::with_id).collect();
		Self {
			contents,
			executable,
			references,
		}
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Handle> {
		let contents = self.contents.handle().clone().into();
		let references = self
			.references
			.iter()
			.map(|reference| reference.clone().into());
		std::iter::once(contents).chain(references).collect()
	}
}

impl Data {
	#[must_use]
	pub fn children(&self) -> Vec<object::Id> {
		std::iter::once(self.contents.into())
			.chain(self.references.iter().copied().map(Into::into))
			.collect()
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

	#[must_use]
	pub fn build(self) -> File {
		File::new(self.contents, self.executable, self.references)
	}
}
