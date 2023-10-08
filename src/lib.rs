#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::redundant_pattern)]

pub use self::client::Client;
#[cfg(feature = "server")]
pub use self::server::Server;
pub use self::{
	artifact::Artifact,
	blob::Blob,
	build::Build,
	bytes::Bytes,
	checksum::Checksum,
	directory::Directory,
	error::{Error, Result, WrapErr},
	file::File,
	id::Id,
	module::Module,
	package::Package,
	path::{Relpath, Subpath},
	placeholder::Placeholder,
	symlink::Symlink,
	system::System,
	target::Target,
	template::Template,
	value::Value,
};

pub mod api;
pub mod artifact;
pub mod blob;
pub mod build;
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
pub mod lsp;
#[cfg(feature = "server")]
pub mod migrations;
pub mod module;
pub mod object;
pub mod package;
pub mod path;
pub mod placeholder;
pub mod runtime;
#[cfg(feature = "server")]
pub mod server;
pub mod symlink;
pub mod system;
pub mod target;
pub mod template;
pub mod util;
pub mod value;
// #[cfg(feature = "server")]
// pub mod vfs;
