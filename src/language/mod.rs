pub use self::{
	diagnostics::{Diagnostic, Severity},
	server::Server,
};

pub mod check;
pub mod completion;
pub mod definition;
pub mod diagnostics;
pub mod doc;
pub mod format;
pub mod hover;
pub mod location;
pub mod metadata;
pub mod references;
pub mod rename;
pub mod server;
pub mod service;
pub mod symbols;
