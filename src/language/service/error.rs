use super::SOURCE_MAP;
use crate::language::Position;
use sourcemap::SourceMap;
use std::sync::Arc;
use thiserror::Error;

/// A language service error.
#[derive(Clone, Debug, Error)]
pub struct Error {
	pub message: String,
	pub stack_trace: Option<StackTrace>,
	pub source: Option<Arc<Error>>,
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

impl Error {
	#[allow(clippy::too_many_lines)]
	pub fn new(scope: &mut v8::HandleScope, exception: v8::Local<v8::Value>) -> Error {
		#[derive(Debug, serde::Deserialize)]
		#[serde(rename_all = "camelCase")]
		struct V8StackTrace {
			call_sites: Vec<V8CallSite>,
		}

		#[allow(dead_code, clippy::struct_excessive_bools)]
		#[derive(Debug, serde::Deserialize)]
		#[serde(rename_all = "camelCase")]
		struct V8CallSite {
			type_name: Option<String>,
			function_name: Option<String>,
			method_name: Option<String>,
			file_name: Option<String>,
			line_number: Option<u32>,
			column_number: Option<u32>,
			is_eval: bool,
			is_native: bool,
			is_constructor: bool,
			is_async: bool,
			is_promise_all: bool,
			// is_promise_any: bool,
			promise_index: Option<u32>,
		}

		// Get the message.
		let message = v8::Exception::create_message(scope, exception)
			.get(scope)
			.to_rust_string_lossy(scope);

		// Get the stack trace.
		let stack_string =
			v8::String::new_external_onebyte_static(scope, "stack".as_bytes()).unwrap();
		let stack_trace = if let Some(stack) = exception
			.is_native_error()
			.then(|| exception.to_object(scope).unwrap())
			.and_then(|exception| exception.get(scope, stack_string.into()))
			.and_then(|value| serde_v8::from_v8::<V8StackTrace>(scope, value).ok())
		{
			let stack_frames = stack
				.call_sites
				.iter()
				.map(|call_site| {
					// Get the location.
					let line = call_site.line_number? - 1;
					let character = call_site.column_number?;
					let position = Position { line, character };

					// Apply the source map if it is available.
					let source_map = Some(SourceMap::from_slice(SOURCE_MAP).unwrap());
					let location = if let Some(source_map) = source_map.as_ref() {
						let token = source_map
							.lookup_token(position.line, position.character)
							.unwrap();
						let path = token.get_source().unwrap();
						let path = path.strip_prefix("../").unwrap().to_owned();
						let position = Position {
							line: token.get_src_line(),
							character: token.get_src_col(),
						};
						Location {
							source: Some(path),
							position,
						}
					} else {
						Location {
							source: None,
							position,
						}
					};

					Some(location)
				})
				.map(|location| StackFrame { location })
				.collect();

			// Create the stack trace.
			Some(StackTrace { stack_frames })
		} else {
			None
		};

		// Get the source.
		let cause_string =
			v8::String::new_external_onebyte_static(scope, "cause".as_bytes()).unwrap();
		let source = if let Some(cause) = exception
			.is_native_error()
			.then(|| exception.to_object(scope).unwrap())
			.and_then(|exception| exception.get(scope, cause_string.into()))
			.and_then(|value| value.to_object(scope))
		{
			let error = Error::new(scope, cause.into());
			Some(Arc::new(error))
		} else {
			None
		};

		// Create the error.
		Self {
			message,
			stack_trace,
			source,
		}
	}
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
