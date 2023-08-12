pub use self::{builder::Builder, data::Data, error::Error};
use crate::{
	block::Block,
	checksum::Checksum,
	error::{Result, WrapErr},
	id::Id,
	target::{from_v8, FromV8, ToV8},
};
use url::Url;

mod builder;
mod data;
#[cfg(feature = "evaluate")]
mod download;
mod error;
mod new;
pub mod unpack;

#[derive(Clone, Debug)]
pub struct Resource {
	/// The resource's block.
	block: Block,

	/// The URL to download from.
	url: Url,

	/// The format to unpack the download with.
	unpack: Option<unpack::Format>,

	/// A checksum of the downloaded file.
	checksum: Option<Checksum>,

	/// If this flag is set, then the download will succeed without a checksum.
	unsafe_: bool,
}

impl Resource {
	#[must_use]
	pub fn block(&self) -> &Block {
		&self.block
	}

	#[must_use]
	pub fn id(&self) -> Id {
		self.block().id()
	}
}

impl std::cmp::PartialEq for Resource {
	fn eq(&self, other: &Self) -> bool {
		self.id() == other.id()
	}
}

impl std::cmp::Eq for Resource {}

impl std::cmp::PartialOrd for Resource {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.id().partial_cmp(&other.id())
	}
}

impl std::cmp::Ord for Resource {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.id().cmp(&other.id())
	}
}

impl std::hash::Hash for Resource {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.id().hash(state);
	}
}

impl ToV8 for Resource {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		// Get the global.
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg_string = v8::String::new(scope, "tg").unwrap();
		let tg = global.get(scope, tg_string.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		// Get the resource class.
		let resource_string = v8::String::new(scope, "Resource").unwrap();
		let resource = tg.get(scope, resource_string.into()).unwrap();
		let resource = resource.to_object(scope).unwrap();
		let constructor = v8::Local::<v8::Function>::try_from(resource).unwrap();

		// Create the resource constructor arg.
		let arg = v8::Object::new(scope);

		let key = v8::String::new(scope, "block").unwrap();
		let value = self.block().to_v8(scope)?;
		arg.set(scope, key.into(), value.into());

		let key = v8::String::new(scope, "url").unwrap();
		let value = self.url.to_string().to_v8(scope)?;
		arg.set(scope, key.into(), value.into());

		let key = v8::String::new(scope, "unpack").unwrap();
		let value = self.unpack.as_ref().map(ToString::to_string).to_v8(scope)?;
		arg.set(scope, key.into(), value.into());

		let key = v8::String::new(scope, "checksum").unwrap();
		let value = self.checksum.to_v8(scope)?;
		arg.set(scope, key.into(), value.into());

		let key = v8::String::new(scope, "unsafe").unwrap();
		let value = self.unsafe_.to_v8(scope)?;
		arg.set(scope, key.into(), value.into());

		// Call the constructor.
		let resource = constructor
			.new_instance(scope, &[arg.into()])
			.wrap_err("The constructor failed.")?;

		Ok(resource.into())
	}
}

impl FromV8 for Resource {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_object(scope).wrap_err("Expected an object.")?;

		let block = value
			.get(scope, v8::String::new(scope, "block").unwrap().into())
			.unwrap();
		let block = from_v8(scope, block)?;

		let url = value
			.get(scope, v8::String::new(scope, "url").unwrap().into())
			.unwrap();
		let url: String = from_v8(scope, url)?;
		let url = Url::parse(&url).map_err(crate::error::Error::other)?;

		let unpack = value
			.get(scope, v8::String::new(scope, "unpack").unwrap().into())
			.unwrap();
		let unpack: Option<String> = from_v8(scope, unpack)?;
		let unpack = unpack.map(|unpack| unpack.parse().unwrap());

		let checksum = value
			.get(scope, v8::String::new(scope, "checksum").unwrap().into())
			.unwrap();
		let checksum = from_v8(scope, checksum)?;

		let unsafe_ = value
			.get(scope, v8::String::new(scope, "unsafe").unwrap().into())
			.unwrap();
		let unsafe_ = from_v8(scope, unsafe_)?;

		Ok(Self {
			block,
			url,
			unpack,
			checksum,
			unsafe_,
		})
	}
}
