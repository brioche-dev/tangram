use super::service;
use crate::{
	error::{return_error, Result},
	instance::Instance,
	module::Module,
};
use std::sync::Arc;

impl Module {
	pub async fn format(tg: &Arc<Instance>, text: String) -> Result<String> {
		// Create the language service request.
		let request = service::Request::Format(service::format::Request { text });

		// Handle the language service request.
		let response = tg.handle_language_service_request(request).await?;

		// Get the response.
		let service::Response::Format(response) = response else {
			return_error!("Unexpected response type.");
		};

		Ok(response.text)
	}
}

#[cfg(test)]
mod tests {
	use crate::instance::{Instance, Options};
	use crate::module::Module;
	use once_cell::sync::Lazy;
	use std::sync::Arc;
	use tokio::sync::Semaphore;

	static SEMAPHORE: Lazy<Arc<Semaphore>> = Lazy::new(|| Arc::new(Semaphore::new(1)));

	macro_rules! test {
		($before:expr, $after:expr$(,)?) => {{
			// Get a permit.
			let _permit = SEMAPHORE.clone().acquire_owned().await.unwrap();

			// Create the instance.
			let temp_dir = tempfile::TempDir::new().unwrap();
			let path = temp_dir.path().to_owned();
			let tg = Arc::new(Instance::new(path, Options::default()).await.unwrap());

			// Test.
			let left = Module::format(&tg, indoc::indoc!($before).to_owned())
				.await
				.unwrap();
			let right = indoc::indoc!($after);
			pretty_assertions::assert_eq!(left, right);
		}};
	}

	#[tokio::test]
	async fn test_basic_formatting() {
		test!(
			r#"
				export default await tg.function(() => {
				return "Hello, world!"
				});
			"#,
			r#"
				export default await tg.function(() => {
					return "Hello, world!";
				});
			"#,
		);
	}

	#[tokio::test]
	async fn test_sort_named_imports() {
		test!(
			r#"
				import { foo, buzz, fizz } from "tangram:foo";
			"#,
			r#"
				import { buzz, fizz, foo } from "tangram:foo";
			"#,
		);
	}

	#[tokio::test]
	async fn test_sort_import_statements() {
		test!(
			r#"
				import * as std from "tangram:std";
				import thing from "./asdf.tg";
				import { foo, bar } from "tangram:foo";
				import zlib from "tangram:zlib";
				import bar from "tangram:bar";

				export default await tg.function(() => {
					return "Hello, world!";
				});
			"#,
			r#"
				import thing from "./asdf.tg";
				import bar from "tangram:bar";
				import { foo, bar } from "tangram:foo";
				import * as std from "tangram:std";
				import zlib from "tangram:zlib";

				export default await tg.function(() => {
					return "Hello, world!";
				});
			"#,
		);

		test!(
			r#"
				import {
					foo1, foo2,
				} from "tangram:foo";
				import {
					bar1,
					bar2,
					bar3,
				} from "tangram:bar";
				import {
					baz1,
				} from "tangram:baz";

				export default await tg.function(() => {
					return "Hello, world!";
				});
			"#,
			r#"
				import {
					bar1,
					bar2,
					bar3,
				} from "tangram:bar";
				import {
					baz1,
				} from "tangram:baz";
				import {
					foo1, foo2,
				} from "tangram:foo";

				export default await tg.function(() => {
					return "Hello, world!";
				});
			"#,
		);
	}

	#[tokio::test]
	async fn test_reindent_multi_line_top_level() {
		// At the top-level, a template should be indented by one level.
		test!(
			r#"
				t`
				foo
				bar
				`;
			"#,
			r#"
				t`
					foo
					bar
				`;
			"#,
		);

		// At the top-level, a template should remove extra indentation so there's one level of indentation.
		test!(
			r#"
				t`
						foo
						bar
				`;
			"#,
			r#"
				t`
					foo
					bar
				`;
			"#
		);

		// Indenting a top-level exported template shouldn't indent the closing backtick.
		test!(
			r#"
				export let x = t`
					`;
			"#,
			r#"
				export let x = t`
				`;
			"#
		);
	}

	#[tokio::test]
	async fn test_reindent_multi_line_nested() {
		// When nested inside a function, a template should be indented to match the indentation of the template start plus one level.
		test!(
			r#"
				import * as std from "tangram:std";

				type Args = {
					target: tg.System;
				};

				export default await tg.function(async ({ target }: Args) => {
					return std.run(
						t`
					echo "hello world" > ${tg.output}
					echo "hi"
						`,
						{ target },
					)
				});
			"#,
			r#"
				import * as std from "tangram:std";

				type Args = {
					target: tg.System;
				};

				export default await tg.function(async ({ target }: Args) => {
					return std.run(
						t`
							echo "hello world" > ${tg.output}
							echo "hi"
						`,
						{ target },
					)
				});
			"#,
		);

		// When nested inside a function, extra indentation should be removed so it matches the indentation of the template start plus one level.
		test!(
			r#"
				import * as std from "tangram:std";

				type Args = {
					target: tg.System;
				};

				export default await tg.function(async ({ target }: Args) => {
					return std.run(
						t`
								echo "hello world" > ${tg.output}
								echo "hi"
						`,
						{ target },
					)
				});
			"#,
			r#"
				import * as std from "tangram:std";

				type Args = {
					target: tg.System;
				};

				export default await tg.function(async ({ target }: Args) => {
					return std.run(
						t`
							echo "hello world" > ${tg.output}
							echo "hi"
						`,
						{ target },
					)
				});
			"#,
		);
	}

	#[tokio::test]
	async fn test_reindent_single_line() {
		// Surrounding whitespace should be stripped for single-line templates at the top-level.
		test!(
			r#"
				t` foo `;
			"#,
			r#"
				t`foo`;
			"#,
		);

		// Surrounding whitespace should be stripped for single-line templates with interpolation.
		test!(
			r#"
				t` foo ${bar} baz `;
			"#,
			r#"
				t`foo ${bar} baz`;
			"#
		);

		// Surrounding whitespace should be stripped for single-line templates nested within a function.
		test!(
			r#"
				import * as std from "tangram:std";

				type Args = {
					target: tg.System;
				};

				export default await tg.function(async ({ target }: Args) => {
					return std.run(
						t` echo "Hello world" > ${tg.output}; echo "hi" `,
						{ target },
					)
				});
			"#,
			r#"
				import * as std from "tangram:std";

				type Args = {
					target: tg.System;
				};

				export default await tg.function(async ({ target }: Args) => {
					return std.run(
						t`echo "Hello world" > ${tg.output}; echo "hi"`,
						{ target },
					)
				});
			"#,
		);
	}

	#[tokio::test]
	async fn test_reindent_multi_line_with_interpolation() {
		// Extra indentation should be added to multi-line templates that aren't indented far enough with interpolation.
		test!(
			r#"
				let jqPrefix = "";
				let json = tg.file('{"foo": "bar"}');
				let jqScript = "'.foo'";
				std.run(
					t`
				mkdir ${tg.output}
				${jqPrefix}${jq} ${jqScript} < ${json}
				${jqPrefix}${jq} ${jqScript} < ${json} > ${tg.output}/output.json
					`,
				);
			"#,
			r#"
				let jqPrefix = "";
				let json = tg.file('{"foo": "bar"}');
				let jqScript = "'.foo'";
				std.run(
					t`
						mkdir ${tg.output}
						${jqPrefix}${jq} ${jqScript} < ${json}
						${jqPrefix}${jq} ${jqScript} < ${json} > ${tg.output}/output.json
					`,
				);
			"#,
		);

		// Extra indentation should be removed from multi-line templates that are indented too far with interpolation.
		test!(
			r#"
				let jqPrefix = "";
				let json = tg.file('{"foo": "bar"}');
				let jqScript = "'.foo'";
				std.run(
					t`
							mkdir ${tg.output}
							${jqPrefix}${jq} ${jqScript} < ${json}
							${jqPrefix}${jq} ${jqScript} < ${json} > ${tg.output}/output.json
					`,
				);
			"#,
			r#"
				let jqPrefix = "";
				let json = tg.file('{"foo": "bar"}');
				let jqScript = "'.foo'";
				std.run(
					t`
						mkdir ${tg.output}
						${jqPrefix}${jq} ${jqScript} < ${json}
						${jqPrefix}${jq} ${jqScript} < ${json} > ${tg.output}/output.json
					`,
				);
			"#,
		);
	}

	#[tokio::test]
	async fn test_reindent_with_inner_indentation() {
		// When there's too much indentation, it should be un-indented, but extra indentation beyond the baseline should be preserved.
		test!(
			r#"
				std.run(
					t`
								if [ -d /usr/local/bin ]; then
									echo "true" > ${tg.output}
								else
									echo "false" > ${tg.output}
								end
					`,
					{ target },
				);
			"#,
			r#"
				std.run(
					t`
						if [ -d /usr/local/bin ]; then
							echo "true" > ${tg.output}
						else
							echo "false" > ${tg.output}
						end
					`,
					{ target },
				);
			"#,
		);

		// When there's not enough indentation, extra indentation should be added so everything has at least the same indentation.
		test!(
			r#"
				std.run(
					t`
				if [ -d /usr/local/bin ]; then
					echo "true" > ${tg.output}
				else
					echo "false" > ${tg.output}
				end
					`,
					{ target },
				);
			"#,
			r#"
				std.run(
					t`
						if [ -d /usr/local/bin ]; then
							echo "true" > ${tg.output}
						else
							echo "false" > ${tg.output}
						end
					`,
					{ target },
				);
			"#,
		);
	}

	#[tokio::test]
	#[allow(clippy::too_many_lines)]
	async fn test_reindent_starts_and_ends_with_a_blank_line() {
		// For a multi-line template, a newline should be added to the start so the first line of the template starts on its own line.
		test!(
			r#"
				std.run(
					t`echo "hello" > ${tg.output}
						echo "world" >> file.txt
					`,
					{ target },
				);
			"#,
			r#"
				std.run(
					t`
						echo "hello" > ${tg.output}
						echo "world" >> file.txt
					`,
					{ target },
				);
			"#,
		);

		// For a multi-line template, a newline should be added to the end so the closing backtick is on its own line.
		test!(
			r#"
				std.run(
					t`
						echo "hello" > ${tg.output}
						echo "world" >> file.txt`,
					{ target },
				);
			"#,
			r#"
				std.run(
					t`
						echo "hello" > ${tg.output}
						echo "world" >> file.txt
					`,
					{ target },
				);
			"#,
		);

		// We may need to add a newline both to the start and the end.
		test!(
			r#"
				std.run(
					t`echo "hello" > ${tg.output}
						echo "world" >> file.txt`,
					{ target },
				);
			"#,
			r#"
				std.run(
					t`
						echo "hello" > ${tg.output}
						echo "world" >> file.txt
					`,
					{ target },
				);
			"#,
		);

		// A newline should be added even if the template starts with interpolation. This is a special case because the first element of the template is a different node here.
		test!(
			r#"
				let echo = "echo";
				std.run(
					t`${echo} "hello" > ${tg.output}
						echo "world" >> file.txt`,
					{ target },
				);
			"#,
			r#"
				let echo = "echo";
				std.run(
					t`
						${echo} "hello" > ${tg.output}
						echo "world" >> file.txt
					`,
					{ target },
				);
			"#,
		);

		// A newline should be added even if the template ends with interpolation. This is a special case because the last element of the template is a different node here.
		test!(
			r#"
				std.run(
					t`echo "hello" > ${tg.output}
						echo "world" >> ${tg.output}`,
					{ target },
				);
			"#,
			r#"
				std.run(
					t`
						echo "hello" > ${tg.output}
						echo "world" >> ${tg.output}
					`,
					{ target },
				);
			"#,
		);

		// A newline should be added even if the template starts _and_ ends with interpolation. Here, we need to add a new node to both the start and end of the template expression.
		test!(
			r#"
				let echo = "echo";
				std.run(
					t`${echo} "hello" > ${tg.output}
						echo "world" >> ${tg.output}`,
					{ target },
				);
			"#,
			r#"
				let echo = "echo";
				std.run(
					t`
						${echo} "hello" > ${tg.output}
						echo "world" >> ${tg.output}
					`,
					{ target },
				);
			"#,
		);

		// A trailing newline should be preserved.
		test!(
			r#"
				import * as std from "tangram:std";

				type Args = {
					target: tg.System;
				};

				export let foo = await tg.function(({ target }: Args) => {
					return std.run(
						t`echo Hello world

						`,
						{ system: target },
					);
				});
			"#,
			r#"
				import * as std from "tangram:std";

				type Args = {
					target: tg.System;
				};

				export let foo = await tg.function(({ target }: Args) => {
					return std.run(
						t`
							echo Hello world

						`,
						{ system: target },
					);
				});
			"#,
		);
	}

	#[tokio::test]
	async fn test_format_default() {
		// Some formatting rules should apply by default.
		test!(
			r#"
				import { foo, buzz, fizz } from "tangram:foo";
				import { bar } from "tangram:bar";

				export let foo = await tg.function(({ target }: Args) => {
					return std.run(
						t`echo Hello world
						echo "hi"
						`,
						{ system: target },
					);
				});
			"#,
			r#"
				import { bar } from "tangram:bar";
				import { buzz, fizz, foo } from "tangram:foo";

				export let foo = await tg.function(({ target }: Args) => {
					return std.run(
						t`
							echo Hello world
							echo "hi"
						`,
						{ system: target },
					);
				});
			"#,
		);
	}
}
