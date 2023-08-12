pub use self::{builder::Builder, data::Data, error::Error};
use crate::{
	block::Block,
	checksum::Checksum,
	error::{Result, WrapErr},
	id::Id,
	system::System,
	target::{from_v8, FromV8, ToV8},
	template::Template,
};
use std::collections::BTreeMap;

#[cfg(feature = "evaluate")]
mod basic;
mod builder;
mod data;
mod error;
#[cfg(all(target_os = "linux", feature = "evaluate"))]
mod linux;
#[cfg(all(target_os = "macos", feature = "evaluate"))]
mod macos;
mod new;
#[cfg(feature = "evaluate")]
mod run;

/// A task.
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, Debug)]
pub struct Task {
	/// The task's block.
	block: Block,

	/// The system to run the task on.
	host: System,

	/// The task's executable.
	executable: Template,

	/// The task's environment variables.
	env: BTreeMap<String, Template>,

	/// The task's command line arguments.
	args: Vec<Template>,

	/// A checksum of the task's output. If a checksum is provided, then unsafe options can be used.
	checksum: Option<Checksum>,

	/// If this flag is set, then unsafe options can be used without a checksum.
	unsafe_: bool,

	/// If this flag is set, then the task will have access to the network. This is an unsafe option.
	network: bool,
}

impl Task {
	#[must_use]
	pub fn id(&self) -> Id {
		self.block().id()
	}

	#[must_use]
	pub fn block(&self) -> &Block {
		&self.block
	}

	#[must_use]
	pub fn system(&self) -> System {
		self.host
	}

	#[must_use]
	pub fn executable(&self) -> &Template {
		&self.executable
	}

	#[must_use]
	pub fn env(&self) -> &BTreeMap<String, Template> {
		&self.env
	}

	#[must_use]
	pub fn args(&self) -> &[Template] {
		&self.args
	}

	#[must_use]
	pub fn checksum(&self) -> &Option<Checksum> {
		&self.checksum
	}

	#[must_use]
	pub fn unsafe_(&self) -> bool {
		self.unsafe_
	}

	#[must_use]
	pub fn network(&self) -> bool {
		self.network
	}
}

impl std::cmp::PartialEq for Task {
	fn eq(&self, other: &Self) -> bool {
		self.id() == other.id()
	}
}

impl std::cmp::Eq for Task {}

impl std::cmp::PartialOrd for Task {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.id().partial_cmp(&other.id())
	}
}

impl std::cmp::Ord for Task {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.id().cmp(&other.id())
	}
}

impl std::hash::Hash for Task {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.id().hash(state);
	}
}

impl ToV8 for Task {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg_string = v8::String::new(scope, "tg").unwrap();
		let tg = global.get(scope, tg_string.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let task_string = v8::String::new(scope, "Task").unwrap();
		let task = tg.get(scope, task_string.into()).unwrap();
		let task = task.to_object(scope).unwrap();
		let constructor = v8::Local::<v8::Function>::try_from(task).unwrap();

		let arg = v8::Object::new(scope);

		let key = v8::String::new(scope, "block").unwrap();
		let value = self.block().to_v8(scope)?;
		arg.set(scope, key.into(), value.into());

		let key = v8::String::new(scope, "host").unwrap();
		let value = self.host.to_v8(scope)?;
		arg.set(scope, key.into(), value.into());

		let key = v8::String::new(scope, "executable").unwrap();
		let value = self.executable.to_v8(scope)?;
		arg.set(scope, key.into(), value.into());

		let key = v8::String::new(scope, "env").unwrap();
		let value = self.env.to_v8(scope)?;
		arg.set(scope, key.into(), value.into());

		let key = v8::String::new(scope, "args").unwrap();
		let value = self.args.to_v8(scope)?;
		arg.set(scope, key.into(), value.into());

		let key = v8::String::new(scope, "checksum").unwrap();
		let value = self.checksum.to_v8(scope)?;
		arg.set(scope, key.into(), value.into());

		let key = v8::String::new(scope, "unsafe").unwrap();
		let value = self.unsafe_.to_v8(scope)?;
		arg.set(scope, key.into(), value.into());

		let key = v8::String::new(scope, "network").unwrap();
		let value = self.network.to_v8(scope)?;
		arg.set(scope, key.into(), value.into());

		// Call the constructor.
		let task = constructor
			.new_instance(scope, &[arg.into()])
			.wrap_err("The constructor failed.")?;

		Ok(task.into())
	}
}

impl FromV8 for Task {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_object(scope).wrap_err("Expected an object.")?;

		let block = value
			.get(scope, v8::String::new(scope, "block").unwrap().into())
			.unwrap();
		let block = from_v8(scope, block)?;

		let host = value
			.get(scope, v8::String::new(scope, "host").unwrap().into())
			.unwrap();
		let host = from_v8(scope, host)?;

		let executable = value
			.get(scope, v8::String::new(scope, "executable").unwrap().into())
			.unwrap();
		let executable = from_v8(scope, executable)?;

		let env = value
			.get(scope, v8::String::new(scope, "env").unwrap().into())
			.unwrap();
		let env = from_v8(scope, env)?;

		let args = value
			.get(scope, v8::String::new(scope, "args").unwrap().into())
			.unwrap();
		let args = from_v8(scope, args)?;

		let checksum = value
			.get(scope, v8::String::new(scope, "checksum").unwrap().into())
			.unwrap();
		let checksum: Option<Checksum> = from_v8(scope, checksum)?;

		let unsafe_ = value
			.get(scope, v8::String::new(scope, "unsafe").unwrap().into())
			.unwrap();
		let unsafe_ = from_v8(scope, unsafe_)?;

		let network = value
			.get(scope, v8::String::new(scope, "network").unwrap().into())
			.unwrap();
		let network = from_v8(scope, network)?;

		Ok(Self {
			block,
			host,
			executable,
			env,
			args,
			checksum,
			unsafe_,
			network,
		})
	}
}
