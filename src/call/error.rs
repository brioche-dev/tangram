use crate::{language::Position, module};
use std::sync::Arc;
use thiserror::Error;

/// An error from a call.
#[derive(Clone, Debug, Error, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Error {
	pub message: String,
	pub location: Option<Location>,
	pub stack_trace: Option<StackTrace>,
	pub source: Option<Arc<crate::error::Error>>,
}

/// A stack trace.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTrace {
	pub stack_frames: Vec<StackFrame>,
}

/// A stack frame.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackFrame {
	pub location: Option<Location>,
}

/// A source location.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
	pub source: Source,
	pub position: Position,
}

/// A source.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum Source {
	#[serde(rename = "global")]
	Global(Option<String>),

	#[serde(rename = "module")]
	Module(module::Identifier),
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		// Write the message.
		write!(f, "{}", self.message)?;

		// // Write the location.
		// if let Some(location) = &self.location {
		// 	writeln!(f)?;
		// 	write!(f, "  {location}")?;
		// }

		// Write the stack trace.
		if let Some(stack_trace) = &self.stack_trace {
			write!(f, "{stack_trace}")?;
		}

		Ok(())
	}
}

impl std::fmt::Display for StackTrace {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for stack_frame in &self.stack_frames {
			writeln!(f)?;
			write!(f, "  {stack_frame}")?;
		}
		Ok(())
	}
}

impl std::fmt::Display for StackFrame {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if let Some(location) = &self.location {
			write!(f, "{location}")?;
		} else {
			write!(f, "[unknown]")?;
		}
		Ok(())
	}
}

impl std::fmt::Display for Location {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let source = &self.source;
		let line = self.position.line + 1;
		let character = self.position.character + 1;
		write!(f, "{source}:{line}:{character}")?;
		Ok(())
	}
}

impl std::fmt::Display for Source {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Source::Global(path) => {
				let path = path.as_deref().unwrap_or("[unknown]");
				write!(f, "global:{path}")?;
			},

			Source::Module(module_identifier) => {
				write!(f, "{module_identifier}")?;
			},
		}
		Ok(())
	}
}
