use crate::module::position::Position;
use std::sync::Arc;
use thiserror::Error;

/// A language service error.
#[derive(
	Clone,
	Debug,
	Error,
	serde::Serialize,
	serde::Deserialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(rename_all = "camelCase")]
pub struct Error {
	#[tangram_serialize(id = 0)]
	pub message: String,
	#[tangram_serialize(id = 1)]
	pub stack_trace: Option<StackTrace>,
	#[tangram_serialize(id = 2)]
	pub source: Option<Arc<crate::error::Error>>,
}

/// A stack trace.
#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(rename_all = "camelCase")]
pub struct StackTrace {
	#[tangram_serialize(id = 0)]
	pub stack_frames: Vec<StackFrame>,
}

/// A stack frame.
#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(rename_all = "camelCase")]
pub struct StackFrame {
	#[tangram_serialize(id = 0)]
	pub location: Option<Location>,
}

/// A source location.
#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(rename_all = "camelCase")]
pub struct Location {
	#[tangram_serialize(id = 0)]
	pub source: Option<String>,
	#[tangram_serialize(id = 1)]
	pub position: Position,
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		// Write the message.
		write!(f, "{}", self.message)?;

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
		let source = self.source.as_deref().unwrap_or("[unknown]");
		let line = self.position.line + 1;
		let character = self.position.character + 1;
		write!(f, "{source}:{line}:{character}")?;
		Ok(())
	}
}
