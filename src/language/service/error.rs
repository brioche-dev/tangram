use crate::language::position::Position;
use std::sync::Arc;
use thiserror::Error;

/// A language service error.
#[derive(Clone, Debug, Error)]
pub struct Error {
	pub message: String,
	pub stack_trace: Option<StackTrace>,
	pub source: Option<Arc<crate::error::Error>>,
}

/// A stack trace.
#[derive(Clone, Debug)]
pub struct StackTrace {
	pub stack_frames: Vec<StackFrame>,
}

/// A stack frame.
#[derive(Clone, Debug)]
pub struct StackFrame {
	pub location: Option<Location>,
}

/// A source location.
#[derive(Clone, Debug)]
pub struct Location {
	pub source: Option<String>,
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
