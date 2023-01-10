use super::context::ContextState;
use num::ToPrimitive;
use std::fmt::Write;

/// Render an exception to a string. The string will include the exception's message and a stack trace.
pub fn render(
	scope: &mut v8::HandleScope,
	context_state: &ContextState,
	exception: v8::Local<v8::Value>,
) -> String {
	let mut string = String::new();

	// Write the exception message.
	let message = exception
		.to_string(scope)
		.unwrap()
		.to_rust_string_lossy(scope);
	writeln!(string, "{message}").unwrap();

	// Write the stack trace if one is available.
	if let Some(stack_trace) = v8::Exception::get_stack_trace(scope, exception) {
		// Write the stack trace one frame at a time.
		for i in 0..stack_trace.get_frame_count() {
			// Retrieve the URL, line, and column.
			let stack_trace_frame = stack_trace.get_frame(scope, i).unwrap();
			let url = stack_trace_frame
				.get_script_name(scope)
				.unwrap()
				.to_rust_string_lossy(scope)
				.parse()
				.unwrap();
			let line = stack_trace_frame.get_line_number().to_u32().unwrap() - 1;
			let column = stack_trace_frame.get_column().to_u32().unwrap() - 1;

			// Apply a source map if one is available.
			let (line, column) = context_state
				.modules
				.borrow()
				.iter()
				.find(|module| module.url == url)
				.and_then(|module| module.source_map.as_ref())
				.map_or((line, column), |source_map| {
					let token = source_map.lookup_token(line, column).unwrap();
					let line = token.get_src_line();
					let column = token.get_src_col();
					(line, column)
				});

			// Write the URL, line, and column.
			write!(string, "{url}:{}:{}", line + 1, column + 1).unwrap();

			// Add a newline if this is not the last frame.
			if i < stack_trace.get_frame_count() - 1 {
				writeln!(string).unwrap();
			}
		}
	}

	string
}
