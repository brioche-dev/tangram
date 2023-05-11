pub use self::{
	diagnostics::{Diagnostic, Severity},
	doc::Doc,
	location::Location,
	position::Position,
	range::Range,
};

pub mod analyze;
pub mod check;
pub mod completion;
pub mod definition;
pub mod diagnostics;
pub mod doc;
pub mod format;
pub mod hover;
pub mod location;
pub mod metadata;
pub mod position;
pub mod range;
pub mod references;
pub mod rename;
pub mod service;
pub mod symbols;
pub mod transpile;
