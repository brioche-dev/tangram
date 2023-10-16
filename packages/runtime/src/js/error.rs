use super::{
	convert::{from_v8, ToV8},
	State,
};
use num::ToPrimitive;
use std::{str::FromStr, sync::Arc};
use tangram_client::{error, Error, Module};

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

pub(super) fn to_exception<'s>(
	scope: &mut v8::HandleScope<'s>,
	error: &Error,
) -> v8::Local<'s, v8::Value> {
	error.to_v8(scope).unwrap()
}

#[allow(clippy::too_many_lines)]
pub(super) fn from_exception<'s>(
	state: &State,
	scope: &mut v8::HandleScope<'s>,
	exception: v8::Local<'s, v8::Value>,
) -> Error {
	let context = scope.get_current_context();
	let global = context.global(scope);
	let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
	let tg = global.get(scope, tg.into()).unwrap();
	let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

	let error = v8::String::new_external_onebyte_static(scope, "Error".as_bytes()).unwrap();
	let error = tg.get(scope, error.into()).unwrap();
	let error = v8::Local::<v8::Function>::try_from(error).unwrap();

	if exception.instance_of(scope, error.into()).unwrap() {
		return from_v8(scope, exception).unwrap();
	}

	// Get the message.
	let message_ = v8::Exception::create_message(scope, exception);
	let message = message_.get(scope).to_rust_string_lossy(scope);

	// Get the location.
	let resource_name = message_
		.get_script_resource_name(scope)
		.and_then(|resource_name| <v8::Local<v8::String>>::try_from(resource_name).ok())
		.map(|resource_name| resource_name.to_rust_string_lossy(scope));
	let line = message_.get_line_number(scope).unwrap().to_u32().unwrap() - 1;
	let column = message_.get_start_column().to_u32().unwrap();
	let location = get_location(state, resource_name.as_deref(), line, column);

	// Get the stack trace.
	let stack = v8::String::new_external_onebyte_static(scope, "stack".as_bytes()).unwrap();
	let stack = if let Some(stack) = exception
		.is_native_error()
		.then(|| exception.to_object(scope).unwrap())
		.and_then(|exception| exception.get(scope, stack.into()))
		.and_then(|value| serde_v8::from_v8::<V8StackTrace>(scope, value).ok())
	{
		let stack = stack
			.call_sites
			.iter()
			.rev()
			.filter_map(|call_site| {
				let file_name = call_site.file_name.as_deref();
				let line: u32 = call_site.line_number? - 1;
				let column: u32 = call_site.column_number?;
				let location = get_location(state, file_name, line, column)?;
				Some(location)
			})
			.collect();
		Some(stack)
	} else {
		None
	};

	// Get the source.
	let cause_string = v8::String::new_external_onebyte_static(scope, "cause".as_bytes()).unwrap();
	let source = if let Some(cause) = exception
		.is_native_error()
		.then(|| exception.to_object(scope).unwrap())
		.and_then(|exception| exception.get(scope, cause_string.into()))
		.and_then(|value| value.to_object(scope))
	{
		let error = from_exception(state, scope, cause.into());
		Some(Arc::new(error))
	} else {
		None
	};

	// Create the error.
	error::Message {
		message,
		location,
		stack,
		source,
	}
	.into()
}

fn get_location(
	state: &State,
	file: Option<&str>,
	line: u32,
	column: u32,
) -> Option<error::Location> {
	if file.map_or(false, |resource_name| resource_name == "[runtime]") {
		if let Some(global_source_map) = state.global_source_map.as_ref() {
			let token = global_source_map.lookup_token(line, column).unwrap();
			let file = token.get_source().unwrap().to_owned();
			let line = token.get_src_line();
			let column = token.get_src_col();
			Some(error::Location { file, line, column })
		} else {
			None
		}
	} else if let Some(module) = file.and_then(|resource_name| Module::from_str(resource_name).ok())
	{
		let file = module.to_string();
		let modules = state.loaded_modules.borrow();
		let (line, column) = if let Some(source_map) = modules
			.iter()
			.find(|source_map_module| source_map_module.module == module)
			.and_then(|source_map_module| source_map_module.source_map.as_ref())
		{
			let token = source_map.lookup_token(line, column).unwrap();
			(token.get_src_line(), token.get_src_col())
		} else {
			(line, column)
		};
		Some(error::Location { file, line, column })
	} else {
		None
	}
}
