pub use self::data::Data;
use crate::error::WrapErr;
use crate::{
	artifact::Artifact,
	block::Block,
	error::Result,
	id::Id,
	target::{from_v8, FromV8, ToV8},
	template::Template,
};

mod data;
mod new;
mod resolve;

#[derive(Clone, Debug)]
pub struct Symlink {
	/// The symlink's block.
	block: Block,

	/// The symlink's target.
	target: Template,
}

impl Symlink {
	#[must_use]
	pub fn id(&self) -> Id {
		self.block().id()
	}

	#[must_use]
	pub fn block(&self) -> &Block {
		&self.block
	}

	#[must_use]
	pub fn target(&self) -> &Template {
		&self.target
	}

	#[must_use]
	pub fn references(&self) -> Vec<Artifact> {
		self.target.artifacts().cloned().collect()
	}
}

impl std::cmp::PartialEq for Symlink {
	fn eq(&self, other: &Self) -> bool {
		self.id() == other.id()
	}
}

impl std::cmp::Eq for Symlink {}

impl std::cmp::PartialOrd for Symlink {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.id().partial_cmp(&other.id())
	}
}

impl std::cmp::Ord for Symlink {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.id().cmp(&other.id())
	}
}

impl std::hash::Hash for Symlink {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.id().hash(state);
	}
}

impl ToV8 for Symlink {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg_string = v8::String::new(scope, "tg").unwrap();
		let tg = global.get(scope, tg_string.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let symlink_string = v8::String::new(scope, "Symlink").unwrap();
		let symlink = tg.get(scope, symlink_string.into()).unwrap();
		let symlink = symlink.to_object(scope).unwrap();
		let constructor = v8::Local::<v8::Function>::try_from(symlink).unwrap();

		let arg = v8::Object::new(scope);

		let key = v8::String::new(scope, "block").unwrap();
		let value = self.block().to_v8(scope)?;
		arg.set(scope, key.into(), value.into());

		let key = v8::String::new(scope, "target").unwrap();
		let value = self.target.to_v8(scope)?;
		arg.set(scope, key.into(), value.into());

		let symlink = constructor
			.new_instance(scope, &[arg.into()])
			.wrap_err("The constructor failed.")?;

		Ok(symlink.into())
	}
}

impl FromV8 for Symlink {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_object(scope).wrap_err("Expected an object.")?;

		let block = value
			.get(scope, v8::String::new(scope, "block").unwrap().into())
			.unwrap();
		let block = from_v8(scope, block)?;

		let target = value
			.get(scope, v8::String::new(scope, "target").unwrap().into())
			.unwrap();
		let target = from_v8(scope, target)?;

		Ok(Self { block, target })
	}
}
