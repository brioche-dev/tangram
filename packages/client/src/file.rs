use crate::{artifact, blob, object, Artifact, Blob, Client, Result, WrapErr};
use bytes::Bytes;

crate::id!(File);
crate::handle!(File);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Id(crate::Id);

#[derive(Clone, Debug)]
pub struct File(object::Handle);

/// A file value.
#[derive(Clone, Debug)]
pub struct Object {
	/// The file's contents.
	pub contents: Blob,

	/// Whether the file is executable.
	pub executable: bool,

	/// The file's references.
	pub references: Vec<Artifact>,
}

/// File data.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Data {
	pub contents: blob::Id,
	pub executable: bool,
	pub references: Vec<artifact::Id>,
}

impl File {
	#[must_use]
	pub fn new(contents: Blob, executable: bool, references: Vec<Artifact>) -> Self {
		Self(object::Handle::with_object(object::Object::File(Object {
			contents,
			executable,
			references,
		})))
	}

	#[must_use]
	pub fn builder(contents: Blob) -> Builder {
		Builder::new(contents)
	}

	pub async fn contents(&self, client: &dyn Client) -> Result<&Blob> {
		Ok(&self.object(client).await?.contents)
	}

	pub async fn executable(&self, client: &dyn Client) -> Result<bool> {
		Ok(self.object(client).await?.executable)
	}

	pub async fn references(&self, client: &dyn Client) -> Result<&[Artifact]> {
		Ok(self.object(client).await?.references.as_slice())
	}
}

impl Object {
	#[must_use]
	pub fn to_data(&self) -> Data {
		let contents = self.contents.expect_id().clone();
		let executable = self.executable;
		let references = self.references.iter().map(Artifact::expect_id).collect();
		Data {
			contents,
			executable,
			references,
		}
	}

	#[must_use]
	pub fn from_data(data: Data) -> Self {
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
		let contents = self.contents.handle().clone();
		let references = self
			.references
			.iter()
			.map(|reference| reference.handle().clone());
		std::iter::once(contents).chain(references).collect()
	}
}

impl Data {
	pub fn serialize(&self) -> Result<Bytes> {
		serde_json::to_vec(self)
			.map(Into::into)
			.wrap_err("Failed to serialize the data.")
	}

	pub fn deserialize(bytes: &Bytes) -> Result<Self> {
		serde_json::from_reader(bytes.as_ref()).wrap_err("Failed to deserialize the data.")
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Id> {
		std::iter::once(self.contents.clone().into())
			.chain(self.references.iter().cloned().map(Into::into))
			.collect()
	}
}

impl std::fmt::Display for File {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.expect_id())?;
		Ok(())
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
