pub use self::{builder::Builder, data::Data};
use crate::error::WrapErr;
use crate::{
	artifact::Artifact,
	block::Block,
	error::{return_error, Error, Result},
	id::Id,
	instance::Instance,
	target::{from_v8, FromV8, ToV8},
};
use futures::{stream::FuturesOrdered, TryStreamExt};
use std::collections::BTreeMap;

mod builder;
mod data;
mod get;
mod new;

#[derive(Clone, Debug)]
pub struct Directory {
	block: Block,
	entries: BTreeMap<String, Block>,
}

impl Directory {
	#[must_use]
	pub fn id(&self) -> Id {
		self.block().id()
	}

	#[must_use]
	pub fn block(&self) -> &Block {
		&self.block
	}

	/// Get the entries.
	pub async fn entries(&self, tg: &Instance) -> Result<BTreeMap<String, Artifact>> {
		let entries = self
			.entries
			.iter()
			.map(|(name, block)| async move {
				let artifact = Artifact::with_block(tg, block.clone()).await?;
				Ok::<_, Error>((name.clone(), artifact))
			})
			.collect::<FuturesOrdered<_>>()
			.try_collect()
			.await?;
		Ok(entries)
	}

	pub async fn try_get_entry(&self, tg: &Instance, name: &str) -> Result<Option<Artifact>> {
		let Some(block) = self.entries.get(name).cloned() else {
			return Ok(None);
		};
		let artifact = Artifact::with_block(tg, block).await?;
		Ok(Some(artifact))
	}

	#[must_use]
	pub fn contains(&self, name: &str) -> bool {
		self.entries.contains_key(name)
	}

	pub fn names(&self) -> impl Iterator<Item = &str> {
		self.entries.keys().map(std::string::String::as_str)
	}

	pub async fn references(&self, tg: &Instance) -> Result<Vec<Artifact>> {
		Ok(self
			.entries(tg)
			.await?
			.into_values()
			.map(|artifact| async move { artifact.references(tg).await })
			.collect::<FuturesOrdered<_>>()
			.try_collect::<Vec<_>>()
			.await?
			.into_iter()
			.flatten()
			.collect())
	}
}

impl std::cmp::PartialEq for Directory {
	fn eq(&self, other: &Self) -> bool {
		self.id() == other.id()
	}
}

impl std::cmp::Eq for Directory {}

impl std::cmp::PartialOrd for Directory {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.id().partial_cmp(&other.id())
	}
}

impl std::cmp::Ord for Directory {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.id().cmp(&other.id())
	}
}

impl std::hash::Hash for Directory {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.id().hash(state);
	}
}

impl ToV8 for Directory {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new(scope, "tg").unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let directory = v8::String::new(scope, "Directory").unwrap();
		let directory = tg.get(scope, directory.into()).unwrap();
		let directory = v8::Local::<v8::Function>::try_from(directory).unwrap();

		let object = directory.new_instance(scope, &[]).unwrap();

		let key = v8::String::new(scope, "block").unwrap();
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.block().to_v8(scope)?;
		object.set_private(scope, key.into(), value.into());

		let key = v8::String::new(scope, "entries").unwrap();
		let key = v8::Private::for_api(scope, Some(key));
		let value = self.entries.to_v8(scope)?;
		object.set_private(scope, key.into(), value.into());

		Ok(object.into())
	}
}

impl FromV8 for Directory {
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

		if !value.instance_of(scope, directory.into()).unwrap() {
			return_error!("Expected a directory.");
		}
		let value = value.to_object(scope).unwrap();

		let block = v8::String::new(scope, "block").unwrap();
		let block = v8::Private::for_api(scope, Some(block));
		let block = value.get_private(scope, block.into()).unwrap();
		let block = from_v8(scope, block)?;

		let entries = v8::String::new(scope, "entries").unwrap();
		let entries = v8::Private::for_api(scope, Some(entries));
		let entries = value.get_private(scope, entries.into()).unwrap();
		let entries = from_v8(scope, entries)?;

		Ok(Self { block, entries })
	}
}
