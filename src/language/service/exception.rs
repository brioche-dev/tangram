use super::error::{Location, StackFrame, StackTrace};
use crate::{error::Error, language::Position, operation};
use num::ToPrimitive;
use sourcemap::SourceMap;
use std::sync::Arc;

pub const SOURCE_MAP: &[u8] = include_bytes!(concat!(
	env!("CARGO_MANIFEST_DIR"),
	"/assets/language_service.js.map"
));

impl Error {
	#[allow(clippy::too_many_lines)]
	pub fn from_language_service_exception(
		scope: &mut v8::HandleScope,
		exception: v8::Local<v8::Value>,
	) -> Error {
		// If the exception is not a native error, then attempt to deserialize it as a Tangram Error.
		if !exception.is_native_error() {
			if let Ok(error) = serde_v8::from_v8(scope, exception) {
				return error;
			}
		}

		// Get the message.
		let message = v8::Exception::create_message(scope, exception)
			.get(scope)
			.to_rust_string_lossy(scope);

		// Get the stack trace.
		let stack_string = v8::String::new(scope, "stack").unwrap();
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
					let file_name = call_site.file_name.as_deref();
					let line = call_site.line_number.unwrap().to_u32().unwrap() - 1;
					let character = call_site.column_number.unwrap().to_u32().unwrap();
					let position = Position { line, character };

					// Apply the source map if it is available.
					let source_map = Some(SourceMap::from_slice(SOURCE_MAP).unwrap());
					let location = if let Some(global_source_map) = source_map.as_ref() {
						let token = global_source_map
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

					// Create the stack frame.
					StackFrame {
						location: Some(location),
					}
				})
				.collect();

			// Create the stack trace.
			Some(StackTrace { stack_frames })
		} else {
			None
		};

		// Get the source.
		let cause_string = v8::String::new(scope, "cause").unwrap();
		let source = if let Some(cause) = exception
			.is_native_error()
			.then(|| exception.to_object(scope).unwrap())
			.and_then(|exception| exception.get(scope, cause_string.into()))
			.and_then(|value| value.to_object(scope))
		{
			let error = Error::from_language_service_exception(scope, cause.into());
			Some(Arc::new(error))
		} else {
			None
		};

		// Create the error.
		Error::LanguageService(super::error::Error {
			message,
			stack_trace,
			source,
		})
	}

	pub fn to_exception<'s>(&self, scope: &mut v8::HandleScope<'s>) -> v8::Local<'s, v8::Value> {
		serde_v8::to_v8(scope, self).expect("Failed to serialize the error.")
	}
}

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
