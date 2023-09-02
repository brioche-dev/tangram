use super::{
	error::{Location, Source, StackFrame, StackTrace},
	state::State,
};
use crate::{build, error::Error, module::position::Position};
use num::ToPrimitive;
use std::sync::Arc;

impl Error {
	#[allow(clippy::too_many_lines)]
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

		// Get the message.
		let message = v8::Exception::create_message(scope, exception)
			.get(scope)
			.to_rust_string_lossy(scope);

		// Get the location.
		let exception_message = v8::Exception::create_message(scope, exception);
		let resource_name = exception_message
			.get_script_resource_name(scope)
			.and_then(|resource_name| <v8::Local<v8::String>>::try_from(resource_name).ok())
			.map(|resource_name| resource_name.to_rust_string_lossy(scope));
		let line = exception_message
			.get_line_number(scope)
			.unwrap()
			.to_u32()
			.unwrap() - 1;
		let character = exception_message.get_start_column().to_u32().unwrap();
		let position = Position { line, character };
		let location = get_location(state, resource_name.as_deref(), position);

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
					let file_name = call_site.file_name.as_deref();
					let line: u32 = call_site.line_number? - 1;
					let character: u32 = call_site.column_number?;
					let position = Position { line, character };
					let location = get_location(state, file_name, position)?;
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
			let error = Error::from_exception(scope, state, cause.into());
			Some(Arc::new(error))
		} else {
			None
		};

		// Create the error.
		Error::Build(build::Error::Target(super::Error {
			message,
			location,
			stack_trace,
			source,
		}))
	}

	pub fn to_exception<'s>(&self, scope: &mut v8::HandleScope<'s>) -> v8::Local<'s, v8::Value> {
		serde_v8::to_v8(scope, self).expect("Failed to serialize the error.")
	}
}

fn get_location(state: &State, file_name: Option<&str>, position: Position) -> Option<Location> {
	if file_name.map_or(false, |resource_name| resource_name == "[global]") {
		// If the file name is "[global]", then create a location whose source is a module.

		// Apply the global source map if it is available.
		let location = if let Some(global_source_map) = state.global_source_map.as_ref() {
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
				source: Source::Global(Some(path)),
				position,
			}
		} else {
			Location {
				source: Source::Global(None),
				position,
			}
		};

		Some(location)
	} else if let Some(module) = file_name.and_then(|resource_name| resource_name.parse().ok()) {
		// If the file name is a module, then create a location whose source is a module.

		// Apply a source map if one is available.
		let modules = state.modules.borrow();
		let position = if let Some(source_map) = modules
			.iter()
			.find(|source_map_module| source_map_module.module == module)
			.and_then(|source_map_module| source_map_module.source_map.as_ref())
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
		};

		// Create the location.
		Some(Location {
			source: Source::Module(module),
			position,
		})
	} else {
		// Otherwise, the location cannot be determined.
		None
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
