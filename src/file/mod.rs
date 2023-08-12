pub use self::data::Data;
use crate::{
	artifact::Artifact,
	blob::{self, Blob},
	block::Block,
	error::{Error, Result, WrapErr},
	id::Id,
	instance::Instance,
	target::{from_v8, FromV8, ToV8},
};
use futures::{stream::FuturesOrdered, TryStreamExt};

mod builder;
mod data;
mod new;

/// A file.
#[derive(Clone, Debug)]
pub struct File {
	/// The file's block.
	block: Block,

	/// The file's contents.
	contents: Block,

	/// Whether the file is executable.
	executable: bool,

	/// The file's references.
	references: Vec<Block>,
}

impl File {
	#[must_use]
	pub fn id(&self) -> Id {
		self.block().id()
	}

	#[must_use]
	pub fn block(&self) -> &Block {
		&self.block
	}

	pub async fn contents(&self, tg: &Instance) -> Result<Blob> {
		Blob::with_block(tg, self.contents.clone()).await
	}

	#[must_use]
	pub fn executable(&self) -> bool {
		self.executable
	}

	pub async fn references(&self, tg: &Instance) -> Result<Vec<Artifact>> {
		let references = self
			.references
			.iter()
			.cloned()
			.map(|block| async move {
				let artifact = Artifact::with_block(tg, block).await?;
				Ok::<_, Error>(artifact)
			})
			.collect::<FuturesOrdered<_>>()
			.try_collect()
			.await?;
		Ok(references)
	}

	pub async fn reader(&self, tg: &Instance) -> Result<blob::Reader> {
		Ok(self.contents(tg).await?.reader(tg))
	}

	pub async fn size(&self, tg: &Instance) -> Result<u64> {
		Ok(self.contents(tg).await?.size())
	}
}

impl std::cmp::PartialEq for File {
	fn eq(&self, other: &Self) -> bool {
		self.id() == other.id()
	}
}

impl std::cmp::Eq for File {}

impl std::cmp::PartialOrd for File {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.id().partial_cmp(&other.id())
	}
}

impl std::cmp::Ord for File {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.id().cmp(&other.id())
	}
}

impl std::hash::Hash for File {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.id().hash(state);
	}
}

impl ToV8 for File {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg_string = v8::String::new(scope, "tg").unwrap();
		let tg = global.get(scope, tg_string.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let file_string = v8::String::new(scope, "File").unwrap();
		let file = tg.get(scope, file_string.into()).unwrap();
		let file = file.to_object(scope).unwrap();
		let constructor = v8::Local::<v8::Function>::try_from(file).unwrap();

		let arg = v8::Object::new(scope);

		let key = v8::String::new(scope, "block").unwrap();
		let value = self.block().to_v8(scope)?;
		arg.set(scope, key.into(), value.into());

		let key = v8::String::new(scope, "contents").unwrap();
		let value = self.contents.to_v8(scope)?;
		arg.set(scope, key.into(), value.into());

		let key = v8::String::new(scope, "executable").unwrap();
		let value = self.executable.to_v8(scope)?;
		arg.set(scope, key.into(), value.into());

		let key = v8::String::new(scope, "references").unwrap();
		let value = self.references.to_v8(scope)?;
		arg.set(scope, key.into(), value.into());

		// Call the constructor.
		let file = constructor
			.new_instance(scope, &[arg.into()])
			.wrap_err("The constructor failed.")?;

		Ok(file.into())
	}
}

impl FromV8 for File {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_object(scope).wrap_err("Expected an object.")?;

		let block = value
			.get(scope, v8::String::new(scope, "block").unwrap().into())
			.unwrap();
		let block = from_v8(scope, block)?;

		let contents = value
			.get(scope, v8::String::new(scope, "contents").unwrap().into())
			.unwrap();
		let contents = from_v8(scope, contents)?;

		let executable = value
			.get(scope, v8::String::new(scope, "executable").unwrap().into())
			.unwrap();
		let executable = from_v8(scope, executable)?;

		let references = value
			.get(scope, v8::String::new(scope, "references").unwrap().into())
			.unwrap();
		let references = from_v8(scope, references)?;

		Ok(Self {
			block,
			contents,
			executable,
			references,
		})
	}
}
