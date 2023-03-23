use crate::language::Location;
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

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		// Write the message.
		write!(f, "{}", self.message)?;

		// Write the location.
		if let Some(location) = &self.location {
			writeln!(f)?;
			write!(
				f,
				"  {}:{}:{}:{}:{}",
				location.module_identifier,
				location.range.start.line + 1,
				location.range.start.character + 1,
				location.range.end.line + 1,
				location.range.end.character + 1,
			)?;
		}

		// Write the stack trace.
		if let Some(stack_trace) = &self.stack_trace {
			write!(f, "{stack_trace}")?;
		}

		Ok(())
	}
}

impl std::fmt::Display for StackTrace {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for location in self
			.stack_frames
			.iter()
			.filter_map(|stack_frame| stack_frame.location.as_ref())
		{
			writeln!(f)?;
			write!(
				f,
				"  {}:{}:{}:{}:{}",
				location.module_identifier,
				location.range.start.line + 1,
				location.range.start.character + 1,
				location.range.end.line + 1,
				location.range.end.character + 1,
			)?;
		}
		Ok(())
	}
}
