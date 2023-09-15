#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_safety_doc)]

#[cfg(feature = "client")]
pub use self::client::Client;
#[cfg(feature = "server")]
pub use self::server::Server;
pub use self::{
	array::Handle as Array,
	artifact::Handle as Artifact,
	blob::Handle as Blob,
	bool::Handle as Bool,
	build::Handle as Build,
	bytes::Handle as Bytes,
	checksum::Checksum,
	directory::Handle as Directory,
	error::{Error, Result, WrapErr},
	file::Handle as File,
	id::Id,
	kind::Kind,
	module::Module,
	null::Handle as Null,
	number::Handle as Number,
	object::Handle as Object,
	package::Handle as Package,
	placeholder::Handle as Placeholder,
	relpath::Handle as Relpath,
	resource::Handle as Resource,
	rid::Rid,
	string::Handle as String,
	subpath::Handle as Subpath,
	symlink::Handle as Symlink,
	system::System,
	target::Handle as Target,
	task::Handle as Task,
	template::Handle as Template,
	value::{Handle, Value},
};

pub mod array;
pub mod artifact;
pub mod blob;
pub mod bool;
pub mod build;
pub mod evaluate;
// pub mod pull;
// pub mod push;
pub mod bundle;
pub mod bytes;
pub mod checkin;
pub mod checkout;
pub mod checksum;
#[cfg(feature = "server")]
pub mod clean;
#[cfg(feature = "client")]
pub mod client;
pub mod directory;
#[cfg(feature = "server")]
pub mod document;
pub mod error;
pub mod evaluation;
pub mod file;
pub mod id;
pub mod kind;
#[cfg(feature = "server")]
pub mod language;
#[cfg(feature = "server")]
pub mod migrations;
pub mod module;
pub mod null;
pub mod number;
pub mod object;
pub mod package;
pub mod placeholder;
pub mod relpath;
pub mod resource;
pub mod rid;
#[cfg(feature = "server")]
pub mod serve;
#[cfg(feature = "server")]
pub mod server;
pub mod string;
pub mod subpath;
pub mod symlink;
pub mod system;
pub mod target;
pub mod task;
pub mod temp;
pub mod template;
pub mod util;
pub mod value;
