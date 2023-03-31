pub use self::{
	diagnostics::{Diagnostic, Severity},
	doc::Doc,
	location::Location,
	position::Position,
	range::Range,
};

mod check;
mod completion;
mod definition;
mod diagnostics;
mod doc;
mod format;
mod hover;
mod imports;
mod location;
mod metadata;
mod position;
mod range;
mod references;
mod rename;
pub mod service;
mod transpile;
