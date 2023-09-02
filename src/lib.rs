#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_safety_doc)]

pub use self::{
	any::Value as Any, array::Value as Array, artifact::Value as Artifact, blob::Value as Blob,
	bool::Value as Bool, bytes::Value as Bytes, directory::Value as Directory, file::Value as File,
	id::Id, instance::Instance, kind::Kind, null::Value as Null, number::Value as Number,
	object::Value as Object, package::Value as Package, placeholder::Value as Placeholder,
	relpath::Value as Relpath, resource::Value as Resource, rid::Rid, string::Value as String,
	subpath::Value as Subpath, symlink::Value as Symlink, system::System, target::Value as Target,
	task::Value as Task, template::Value as Template, value::Value,
};

pub mod any;
pub mod array;
pub mod artifact;
pub mod blob;
pub mod bool;
pub mod build;
// pub mod bundle;
pub mod bytes;
pub mod checkin;
pub mod checkout;
pub mod checksum;
#[cfg(feature = "client")]
pub mod client;
pub mod directory;
#[cfg(feature = "language")]
pub mod document;
pub mod error;
pub mod file;
pub mod id;
pub mod instance;
pub mod kind;
#[cfg(feature = "language")]
pub mod language;
pub mod migrations;
pub mod module;
pub mod null;
pub mod number;
pub mod object;
// pub mod output;
pub mod package;
pub mod placeholder;
pub mod relpath;
pub mod resource;
pub mod rid;
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
