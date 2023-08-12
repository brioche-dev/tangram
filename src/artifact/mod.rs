pub use self::{data::Data, tracker::Tracker};
use crate::{
	block::Block,
	directory::Directory,
	error::{return_error, Result, WrapErr},
	file::File,
	id::Id,
	instance::Instance,
	symlink::Symlink,
	target::{from_v8, FromV8, ToV8},
};
use async_recursion::async_recursion;

mod bundle;
pub mod checkin;
mod checkout;
mod checksum;
mod data;
mod references;
mod tracker;

/// An artifact.
#[derive(Clone, Debug)]
pub enum Artifact {
	/// A directory.
	Directory(Directory),

	/// A file.
	File(File),

	/// A symlink.
	Symlink(Symlink),
}

impl Artifact {
	#[async_recursion]
	pub async fn with_block(tg: &'async_recursion Instance, block: Block) -> Result<Self> {
		let id = block.id();
		let artifact = Self::try_with_block(tg, block)
			.await?
			.wrap_err_with(|| format!(r#"Failed to get the artifact "{id}"."#))?;
		Ok(artifact)
	}

	pub async fn try_with_block(tg: &Instance, block: Block) -> Result<Option<Self>> {
		// Get the data.
		let Some(data) = block.try_get_data(tg).await? else {
			return Ok(None);
		};

		// Deserialize the data.
		let data = Data::deserialize(&*data)?;

		// Create the artifact from the data.
		let artifact = Self::from_data(tg, block, data).await?;

		Ok(Some(artifact))
	}

	#[must_use]
	pub fn id(&self) -> Id {
		self.block().id()
	}

	#[must_use]
	pub fn block(&self) -> &Block {
		match self {
			Self::Directory(directory) => directory.block(),
			Self::File(file) => file.block(),
			Self::Symlink(symlink) => symlink.block(),
		}
	}

	pub async fn store(&self, tg: &Instance) -> Result<()> {
		self.block().store(tg).await
	}
}

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
}

impl From<Directory> for Artifact {
	fn from(directory: Directory) -> Self {
		Self::Directory(directory)
	}
}

impl From<File> for Artifact {
	fn from(file: File) -> Self {
		Self::File(file)
	}
}

impl From<Symlink> for Artifact {
	fn from(symlink: Symlink) -> Self {
		Self::Symlink(symlink)
	}
}

impl std::fmt::Display for Artifact {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Artifact::Directory(directory) => {
				write!(f, r#"(tg.directory {})"#, directory.id())?;
			},
			Artifact::File(file) => {
				write!(f, r#"(tg.file {})"#, file.id())?;
			},
			Artifact::Symlink(symlink) => {
				write!(f, r#"(tg.symlink {})"#, symlink.id())?;
			},
		}
		Ok(())
	}
}

impl std::cmp::PartialEq for Artifact {
	fn eq(&self, other: &Self) -> bool {
		self.id() == other.id()
	}
}

impl std::cmp::Eq for Artifact {}

impl std::cmp::PartialOrd for Artifact {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.id().partial_cmp(&other.id())
	}
}

impl std::cmp::Ord for Artifact {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.id().cmp(&other.id())
	}
}

impl std::hash::Hash for Artifact {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.id().hash(state);
	}
}

impl ToV8 for Artifact {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		match self {
			Self::Directory(directory) => directory.to_v8(scope),
			Self::File(file) => file.to_v8(scope),
			Self::Symlink(symlink) => symlink.to_v8(scope),
		}
	}
}

impl FromV8 for Artifact {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg_string = v8::String::new(scope, "tg").unwrap();
		let tg = global.get(scope, tg_string.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let directory = v8::String::new(scope, "Directory").unwrap();
		let directory = tg.get(scope, directory.into()).unwrap();
		let directory = v8::Local::<v8::Function>::try_from(directory).unwrap();

		let file = v8::String::new(scope, "File").unwrap();
		let file = tg.get(scope, file.into()).unwrap();
		let file = v8::Local::<v8::Function>::try_from(file).unwrap();

		let symlink = v8::String::new(scope, "Symlink").unwrap();
		let symlink = tg.get(scope, symlink.into()).unwrap();
		let symlink = v8::Local::<v8::Function>::try_from(symlink).unwrap();

		let artifact = if value.instance_of(scope, directory.into()).unwrap() {
			Self::Directory(from_v8(scope, value)?)
		} else if value.instance_of(scope, file.into()).unwrap() {
			Self::File(from_v8(scope, value)?)
		} else if value.instance_of(scope, symlink.into()).unwrap() {
			Self::Symlink(from_v8(scope, value)?)
		} else {
			return_error!("Expected a directory, file, or symlink.")
		};

		Ok(artifact)
	}
}
