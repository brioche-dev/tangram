use super::{
	context::State,
	error::{StackFrame, StackTrace},
};
use crate::{
	error::Error,
	language::{Location, Position, Range},
	module, operation,
};
use num::ToPrimitive;
use std::sync::Arc;

impl Error {
	pub fn from_exception(
		scope: &mut v8::HandleScope,
		state: &State,
		exception: v8::Local<v8::Value>,
	) -> Error {
		// If the exception is not a native error, then attempt to deserialize it as a Tangram Error.
		if !exception.is_native_error() {
			if let Ok(error) = serde_v8::from_v8(scope, exception) {
				return error;
			}
		}

		// Get the location.
		let message = v8::Exception::create_message(scope, exception);
		let location = if let Some(resource_name) = message
			.get_script_resource_name(scope)
			.and_then(|resource_name| <v8::Local<v8::String>>::try_from(resource_name).ok())
		{
			// parse the resource name as a module identifier.
			let module_identifier = resource_name.to_rust_string_lossy(scope).parse().unwrap();

			// Get the start and end positions.
			let line = message.get_line_number(scope).unwrap().to_u32().unwrap() - 1;
			let start_character = message.get_start_column().to_u32().unwrap();
			let end_character = message.get_end_column().to_u32().unwrap();
			let start = Position {
				line,
				character: start_character,
			};
			let start = apply_source_map(state, Some(&module_identifier), start);
			let end = Position {
				line,
				character: end_character,
			};
			let end = apply_source_map(state, Some(&module_identifier), end);

			// Create the range.
			let range = Range { start, end };

			// Create the location.
			Some(Location {
				module_identifier,
				range,
			})
		} else {
			None
		};

		// Get the message.
		let message = v8::Exception::create_message(scope, exception)
			.get(scope)
			.to_rust_string_lossy(scope);

		// If the exception is not a native error, then stringify it.
		if !exception.is_native_error() {
			return Error::Operation(operation::Error::Call(super::Error {
				message,
				location,
				stack_trace: None,
				source: None,
			}));
		}

		// At this point, the exception is a native error.
		let exception = exception.to_object(scope).unwrap();

		// Get the stack trace.
		let stack_string = v8::String::new(scope, "stack").unwrap();
		let stack_trace = if let Some(stack) = exception
			.get(scope, stack_string.into())
			.and_then(|value| serde_v8::from_v8::<V8StackTrace>(scope, value).ok())
		{
			let stack_frames = stack
				.call_sites
				.iter()
				.map(|call_site| {
					// Get the location.
					let location = if let Some(module_identifier) = call_site
						.file_name
						.as_ref()
						.and_then(|file_name| file_name.parse().ok())
					{
						// Get the position.
						let line = call_site.line_number.unwrap().to_u32().unwrap() - 1;
						let character = call_site.column_number.unwrap().to_u32().unwrap();
						let position = Position { line, character };
						let position = apply_source_map(state, Some(&module_identifier), position);

						// Create the location.
						Some(Location {
							module_identifier,
							range: Range {
								start: position,
								end: position,
							},
						})
					} else {
						None
					};

					// Create the stack frame.
					StackFrame { location }
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
			.get(scope, cause_string.into())
			.and_then(|value| value.to_object(scope))
		{
			let error = Error::from_exception(scope, state, cause.into());
			Some(Arc::new(error))
		} else {
			None
		};

		// Create the error.
		Error::Operation(operation::Error::Call(super::Error {
			message,
			location,
			stack_trace,
			source,
		}))
	}

	pub fn to_exception<'s>(&self, scope: &mut v8::HandleScope<'s>) -> v8::Local<'s, v8::Value> {
		// Create the exception.
		let message = v8::String::new(scope, "An uncaught exception was thrown.").unwrap();
		let exception = v8::Exception::error(scope, message);
		let exception = exception
			.to_object(scope)
			.expect("Expected the exception to be an object.");

		// Serialize this error and set it to the `cause` field of the exception.
		let cause_key = v8::String::new(scope, "cause").unwrap();
		let cause_value = serde_v8::to_v8(scope, self).expect("Failed to serialize the error.");
		exception.set(scope, cause_key.into(), cause_value);

		exception.into()
	}
}

fn apply_source_map(
	state: &State,
	module_identifier: Option<&module::Identifier>,
	position: Position,
) -> Position {
	let modules = state.modules.borrow();
	if let Some(source_map) = module_identifier
		.and_then(|module_identifier| {
			modules
				.iter()
				.find(|module| module.module_identifier == *module_identifier)
		})
		.and_then(|module| module.source_map.as_ref())
	{
		let token = source_map
			.lookup_token(position.line, position.character)
			.unwrap();
		Position {
			line: token.get_src_line(),
			character: token.get_src_col(),
		}
	} else {
		position
	}
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct V8StackTrace {
	call_sites: Vec<V8CallSite>,
}

#[allow(dead_code, clippy::struct_excessive_bools)]
#[derive(serde::Deserialize)]
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
