use std::fmt::Write;

/// Render an exception to a string. The string will include the exception's message and a stack trace.
pub fn render(scope: &mut v8::HandleScope, exception: v8::Local<v8::Value>) -> String {
	let mut string = String::new();

	// Write the exception message.
	let message = exception
		.to_string(scope)
		.unwrap()
		.to_rust_string_lossy(scope);
	writeln!(&mut string, "{message}").unwrap();

	// Write the stack trace if one is available.
	if let Some(stack_trace) = v8::Exception::get_stack_trace(scope, exception) {
		// Write the stack trace one frame at a time.
		for i in 0..stack_trace.get_frame_count() {
			// Retrieve the line, and column.
			let stack_trace_frame = stack_trace.get_frame(scope, i).unwrap();
			let line = stack_trace_frame.get_line_number();
			let column = stack_trace_frame.get_column();

			// Write the line and column.
			write!(string, "{line}:{column}").unwrap();

			// Add a newline if this is not the last frame.
			if i < stack_trace.get_frame_count() - 1 {
				writeln!(&mut string).unwrap();
			}
		}
	}

	string
}
