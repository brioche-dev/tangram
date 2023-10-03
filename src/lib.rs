#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_safety_doc)]

pub use self::client::Client;
#[cfg(feature = "server")]
pub use self::server::Server;
pub use self::{
	artifact::Artifact,
	blob::Blob,
	bytes::Bytes,
	checksum::Checksum,
	directory::Directory,
	error::{Error, Result, WrapErr},
	file::File,
	id::Id,
	package::Package,
	path::{Relpath, Subpath},
	placeholder::Placeholder,
	run::Run,
	symlink::Symlink,
	system::System,
	task::Task,
	template::Template,
	value::Value,
};

pub mod api;
pub mod artifact;
pub mod blob;
pub mod bundle;
pub mod bytes;
pub mod checkin;
pub mod checkout;
pub mod checksum;
#[cfg(feature = "server")]
pub mod clean;
pub mod client;
pub mod directory;
pub mod error;
pub mod file;
pub mod id;
#[cfg(feature = "server")]
pub mod language;
#[cfg(feature = "server")]
pub mod migrations;
pub mod object;
pub mod package;
pub mod path;
pub mod placeholder;
pub mod run;
#[cfg(feature = "server")]
pub mod server;
pub mod symlink;
pub mod system;
pub mod task;
pub mod temp;
pub mod template;
pub mod util;
pub mod value;
