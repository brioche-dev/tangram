use itertools::Itertools;
use rome_analyze::{Ast, GroupCategory, Rule, RuleAction, RuleGroup, RuleMeta};
use rome_js_syntax::{AnyJsTemplateElement, JsLanguage, JsTemplateExpression};
use rome_rowan::{AstNode, AstNodeList, BatchMutationExt};

pub fn visit_registry<V: rome_analyze::RegistryVisitor<JsLanguage> + ?Sized>(registry: &mut V) {
	registry.record_category::<TangramRuleCategory>();
}

enum TangramRuleCategory {}

impl GroupCategory for TangramRuleCategory {
	type Language = JsLanguage;

	const CATEGORY: rome_analyze::RuleCategory = rome_analyze::RuleCategory::Lint;

	fn record_groups<V: rome_analyze::RegistryVisitor<Self::Language> + ?Sized>(registry: &mut V) {
		registry.record_group::<FormatRules>();
	}
}

enum FormatRules {}

impl RuleGroup for FormatRules {
	type Language = JsLanguage;

	type Category = TangramRuleCategory;

	const NAME: &'static str = "tangramFormat";

	fn record_rules<V: rome_analyze::RegistryVisitor<Self::Language> + ?Sized>(registry: &mut V) {
		registry.record_rule::<TemplateIndentRule>();
	}
}

enum TemplateIndentRule {}

impl RuleMeta for TemplateIndentRule {
	type Group = FormatRules;

	const METADATA: rome_analyze::RuleMetadata =
		rome_analyze::RuleMetadata::new("0.1.0", "templateIndent", "");
}

impl Rule for TemplateIndentRule {
	type Query = Ast<JsTemplateExpression>;

	type State = String;

	type Signals = Option<String>;

	type Options = ();

	fn run(ctx: &rome_analyze::context::RuleContext<Self>) -> Self::Signals {
		let node = ctx.query();
		let root = ctx.root();

		let tag = node.tag()?;
		let tag_name = tag.as_js_identifier_expression()?.name().ok()?;
		if !tag_name.has_name("t") {
			return None;
		}

		let template_start_index = node.syntax().text_trimmed_range().start();
		let template_start_index: usize = template_start_index.into();
		let root_text = root.text();
		let text_to_template_start = &root_text[..template_start_index];
		let template_start_line = text_to_template_start.lines().last().unwrap_or_default();
		let base_indentation = get_indentation(template_start_line);

		if reindent_template(node, base_indentation).is_some() {
			Some(base_indentation.to_string())
		} else {
			None
		}
	}

	fn action(
		ctx: &rome_analyze::context::RuleContext<Self>,
		state: &String,
	) -> Option<RuleAction<JsLanguage>> {
		let root = ctx.root();
		let node = ctx.query();
		let base_indentation = state;

		let new_node = reindent_template(node, base_indentation)?;

		let mut mutation = root.begin();
		mutation.replace_node(node.clone(), new_node);

		Some(RuleAction {
			category: rome_analyze::ActionCategory::QuickFix,
			applicability: rome_diagnostics::Applicability::MaybeIncorrect,
			message: rome_console::markup!("Re-indent Template").to_owned(),
			mutation,
		})
	}
}

fn reindent_template(
	node: &JsTemplateExpression,
	base_indentation: &str,
) -> Option<JsTemplateExpression> {
	// Determine if the template is single-line or multi-line. The template is multi-line if there's at least one chunk that contains a newline.
	let is_multi_line = node
		.elements()
		.iter()
		.filter_map(|element| {
			let chunk_element = element.as_js_template_chunk_element()?;
			let chunk_token = chunk_element.template_chunk_token().ok()?;
			Some(chunk_token)
		})
		.any(|chunk_token| chunk_token.text().contains('\n'));

	if is_multi_line {
		reindent_template_multi_line(node, base_indentation)
	} else {
		reindent_template_single_line(node)
	}
}

fn reindent_template_single_line(node: &JsTemplateExpression) -> Option<JsTemplateExpression> {
	// Construct a new list of template elements, and track if any elements are different.
	let mut new_elements: Vec<AnyJsTemplateElement> = vec![];
	let mut elements_changed = false;

	let num_elements = node.elements().len();
	for (element_index, element) in node.elements().iter().enumerate() {
		let is_first_chunk = element_index == 0;
		let is_last_chunk = element_index == num_elements - 1;

		// Keep a non-chunk element the same (a.k.a something other than a string literal).
		let Some(chunk_element) = element.as_js_template_chunk_element() else {
			new_elements.push(element);
			continue;
		};

		// Keep an element the same if there's an error.
		let Some(chunk_token) = chunk_element.template_chunk_token().ok() else {
			new_elements.push(element);
			continue;
		};

		// Start with the current chunk text.
		let chunk_text = chunk_token.text();
		let mut new_chunk_text = chunk_text;

		// Trim the start for the first chunk.
		// t` foo ${...}` -> t`foo ${...}`
		if is_first_chunk {
			new_chunk_text = new_chunk_text.trim_start();
		}

		// Trim the end for the last chunk.
		// t`${...} foo ` -> t`${...} foo`
		if is_last_chunk {
			new_chunk_text = new_chunk_text.trim_end();
		}

		// If the chunk text didn't change, keep the element the same.
		if new_chunk_text == chunk_text {
			new_elements.push(element);
			continue;
		}

		// Construct the new chunk and add it.
		let new_chunk_element = rome_js_factory::make::js_template_chunk_element(
			rome_js_syntax::JsSyntaxToken::new_detached(
				rome_js_syntax::JsSyntaxKind::TEMPLATE_CHUNK,
				new_chunk_text,
				[],
				[],
			),
		);
		new_elements.push(new_chunk_element.into());
		elements_changed = true;
	}

	if elements_changed {
		// Construct the new template node with the new list of elements, and return `Some` to indicate that the template was re-indented.
		let new_node = node
			.clone()
			.with_elements(rome_js_factory::make::js_template_element_list(
				new_elements,
			));

		Some(new_node)
	} else {
		// Return `None` to indicate that the template was unchanged.
		None
	}
}

#[allow(clippy::too_many_lines)]
fn reindent_template_multi_line(
	node: &JsTemplateExpression,
	base_indentation: &str,
) -> Option<JsTemplateExpression> {
	// Add an extra tab so things get indented by one more level.
	// TODO: Do we want to support other indentation styles?
	let new_template_indentation = format!("{base_indentation}\t");

	// Determine the current indentation of the template by finding the shortest string of ' ' and '\t' characters at the start of each line.
	let current_template_indentation = node
		.elements()
		.iter()
		.filter_map(|element| {
			let chunk_element = element.as_js_template_chunk_element()?;
			let chunk_token = chunk_element.template_chunk_token().ok()?;
			Some(chunk_token)
		})
		.flat_map(|chunk_token| {
			let chunk_lines = chunk_token.text().split('\n');
			let chunk_indentations = chunk_lines.skip(1).filter_map(|line| {
				// Skip empty lines.
				if line.trim().is_empty() {
					return None;
				}

				// Get the indentation.
				let indentation = get_indentation(line);
				Some(indentation.to_string())
			});

			chunk_indentations.collect::<Vec<_>>()
		})
		.min_by_key(String::len)
		.unwrap_or_default();

	// Construct a new list of template elements, and track if any elements are different.
	let mut new_elements: Vec<AnyJsTemplateElement> = vec![];
	let mut elements_changed = false;

	let num_elements = node.elements().len();
	for (element_index, element) in node.elements().iter().enumerate() {
		let is_first_chunk = element_index == 0;
		let is_last_chunk = element_index == num_elements - 1;

		// Try to get the element as a chunk (string literal).
		let Some(chunk_element) = element.as_js_template_chunk_element() else {
			// If the first element isn't a chunk, then that means the template starts with an interpolated expression, so we add a newline so the template content starts on its own line.
			// t`${...}` -> t`\n${...}`
			if is_first_chunk {
				let newline = rome_js_factory::make::js_template_chunk_element(
					rome_js_syntax::JsSyntaxToken::new_detached(
						rome_js_syntax::JsSyntaxKind::TEMPLATE_CHUNK,
						&format!("\n{new_template_indentation}"),
						[],
						[],
					)
				);
				new_elements.push(newline.into());
				elements_changed = true;
			}

			// Add the element as-is to keep the interpolation.
			new_elements.push(element);

			// If the last element isn't a chunk, then that means the template starts with an interpolated expression, so we add a newline so the closing backtick starts on its own line.
			// t`${...}` -> t`${...}\n`
			if is_last_chunk {
				let newline = rome_js_factory::make::js_template_chunk_element(
					rome_js_syntax::JsSyntaxToken::new_detached(
						rome_js_syntax::JsSyntaxKind::TEMPLATE_CHUNK,
						&format!("\n{new_template_indentation}"),
						[],
						[],
					)
				);
				new_elements.push(newline.into());
				elements_changed = true;
			}

			continue;
		};

		// Keep the element the same if there's an error.
		let Some(chunk_token) = chunk_element.template_chunk_token().ok() else {
			new_elements.push(element);
			continue;
		};

		let chunk_text = chunk_token.text();

		// Split on newlines. This works slightly differently than `.lines()` in some cases for blank strings.
		let mut chunk_text_lines = chunk_text.split('\n').peekable();

		// Add a starting newline to this chunk if this is the first chunk and the first line isn't empty. This means that the template starts with a string literal, but starts on the same line as the opening backtick. We want to add a newline so the template content starts on its own line.
		// t`foo${...}` -> t`\nfoo${...}`
		let should_add_start_newline = chunk_text_lines.peek().map_or(false, |first_line| {
			is_first_chunk && !first_line.trim().is_empty()
		});

		// Add an ending newline to this chunk if this is the last chunk.
		// t`${...}foo` -> t`${...}foo\n`
		let should_add_end_newline = is_last_chunk;

		// Adjust the indentation of each line of the chunk.
		let num_chunk_text_lines = chunk_text_lines.clone().count();
		let mut new_chunk_text = chunk_text_lines
			.enumerate()
			.map(|(line_index, line)| {
				let is_first_line = line_index == 0;
				let is_last_line = line_index == num_chunk_text_lines - 1;

				// If this is the first line but not the first chunk, this means that this line is following an interpolated value. We don't want to indent it because the true start of this line is in a different chunk.
				// Example: t`\n${foo} bar\n` ('bar\n' is the chunk in this case; no extra indentation should be added because it's not on its own line).
				if is_first_line && !is_first_chunk {
					return line.to_string();
				}

				// Empty lines should not be indented normally.
				if line.trim().is_empty() {
					// Add one last bit of indentation if this is the last line of the last chunk. This ensures that the closing backtick is indented properly.
					// \tt`\t\t${foo}\n` -> \tt`\t\t${foo}\n\t`
					if is_last_line && !is_last_chunk {
						return new_template_indentation.to_string();
					}

					// ...Otherwise, for normal blank lines, return an empty string. This strips extra trailing whitespace.
					return String::new();
				}

				// Calculate the byte offsets where the current baseline indentation ends and the line content starts. Note that the "content" in this case can still include extra indentation past the baseline that we want to preserve.
				let content_start_after_indentation = line
					.bytes()
					.enumerate()
					.take(current_template_indentation.len())
					.take_while(|&(_, b)| b == b' ' || b == b'\t')
					.last()
					.map_or(0, |(i, _)| i + 1);

				let (_line_indent, line_content) = line.split_at(content_start_after_indentation);

				// Add the new indentation to the start of the line content.
				format!("{new_template_indentation}{line_content}")
			})
			.join("\n");

		// Add a newline to the start of the chunk if necessary.
		if should_add_start_newline {
			new_chunk_text = format!("\n{new_chunk_text}");
		}

		// Add a newline to the end of the chunk if necessary and if it doesn't already end with one.
		if should_add_end_newline {
			if !new_chunk_text.ends_with('\n') {
				new_chunk_text.push('\n');
			}

			// For the last line of the last chunk, add extra indentation so the closing backtick lines up with the opening backtick properly.
			new_chunk_text += base_indentation;
		}

		if new_chunk_text == chunk_text {
			// If the chunk text already has the correct indentation, keep it the same.
			new_elements.push(element);
		} else {
			// Otherwise, building a new template chunk element with the new text.

			let new_chunk_element = rome_js_factory::make::js_template_chunk_element(
				rome_js_syntax::JsSyntaxToken::new_detached(
					rome_js_syntax::JsSyntaxKind::TEMPLATE_CHUNK,
					&new_chunk_text,
					[],
					[],
				),
			);

			elements_changed = true;
			new_elements.push(new_chunk_element.into());
		}
	}

	if elements_changed {
		// Construct the new template node with the new list of elements, and return `Some` to indicate that the template was re-indented.
		let new_node = node
			.clone()
			.with_elements(rome_js_factory::make::js_template_element_list(
				new_elements,
			));

		Some(new_node)
	} else {
		// Return `None` to indicate that the template was unchanged.
		None
	}
}

/// Get a slice of the indentation from a string.
fn get_indentation(line: &str) -> &str {
	let whitespace_end = line
		.bytes()
		.enumerate()
		.take_while(|&(_, b)| b == b' ' || b == b'\t')
		.last()
		.map(|(i, _)| i);

	if let Some(whitespace_end) = whitespace_end {
		&line[..=whitespace_end]
	} else {
		""
	}
}

#[cfg(test)]
mod tests {
	fn reindent_templates(code: &str) -> String {
		let source_type = rome_js_syntax::SourceType::ts();

		let mut ast_root = rome_js_parser::parse(code, source_type).tree();

		let rule_filter = rome_analyze::AnalysisFilter::from_enabled_rules(Some(&[
			rome_analyze::RuleFilter::Rule("tangramFormat", "templateIndent"),
		]));
		crate::compiler::analyze::fix_all(&mut ast_root, rule_filter).expect("Failed to fix rules");

		ast_root.to_string()
	}

	/// Asserts that applying the `templateIndent` rule on the `$before` is equal to `$after`. The expressions `$before` and `$after` should be string literals and are automatically wrapped with [`indoc::indoc!`].
	macro_rules! assert_eq_after_reindent {
		($before:expr, $after:expr) => {
			assert_eq!(
				reindent_templates(indoc::indoc!($before)),
				indoc::indoc!($after),
			)
		};
		($before:expr, $after:expr,) => {
			assert_eq_after_reindent!($before, $after)
		};
	}

	#[test]
	fn test_reindent_multi_line_top_level() {
		// At the top-level, a template should be indented by one level.
		assert_eq_after_reindent!(
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
		assert_eq_after_reindent!(
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
		assert_eq_after_reindent!(
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

	#[test]
	fn test_reindent_multi_line_nested() {
		// When nested inside a function, a template should be indented to match the indentation of the template start plus one level.
		assert_eq_after_reindent!(
			r#"
				import * as std from "tangram:std";

				type Args = {
					target: tg.System;
				};

				export default tg.createTarget(async ({ target }: Args) => {
					return std.bash(
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

				export default tg.createTarget(async ({ target }: Args) => {
					return std.bash(
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
		assert_eq_after_reindent!(
			r#"
				import * as std from "tangram:std";

				type Args = {
					target: tg.System;
				};

				export default tg.createTarget(async ({ target }: Args) => {
					return std.bash(
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

				export default tg.createTarget(async ({ target }: Args) => {
					return std.bash(
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

	#[test]
	fn test_reindent_single_line() {
		// Surrounding whitespace should be stripped for single-line templates at the top-level.
		assert_eq_after_reindent!(
			r#"
				t` foo `;
			"#,
			r#"
				t`foo`;
			"#,
		);

		// Surrounding whitespace should be stripped for single-line templates with interpolation.
		assert_eq_after_reindent!(
			r#"
				t` foo ${bar} baz `;
			"#,
			r#"
				t`foo ${bar} baz`;
			"#
		);

		// Surrounding whitespace should be stripped for single-line templates nested within a function.
		assert_eq_after_reindent!(
			r#"
				import * as std from "tangram:std";

				type Args = {
					target: tg.System;
				};

				export default tg.createTarget(async ({ target }: Args) => {
					return std.bash(
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

			export default tg.createTarget(async ({ target }: Args) => {
				return std.bash(
					t`echo "Hello world" > ${tg.output}; echo "hi"`,
					{ target },
				)
			});
			"#,
		);
	}

	#[test]
	fn test_reindent_multi_line_with_interpolation() {
		// Extra indentation should be added to multi-line templates that aren't indented far enough with interpolation.
		assert_eq_after_reindent!(
			r#"
				let jqPrefix = "";
				let json = tg.file('{"foo": "bar"}');
				let jqScript = "'.foo'";
				std.bash(
					t`
				mkdir ${tg.output}
				${jqPrefix}${jq} ${jqScript} < ${json}
				${jqPrefix}${jq} ${jqScript} < ${json} > ${tg.output}/output.json
					`,
					{ target },
				);
			"#,
			r#"
				let jqPrefix = "";
				let json = tg.file('{"foo": "bar"}');
				let jqScript = "'.foo'";
				std.bash(
					t`
						mkdir ${tg.output}
						${jqPrefix}${jq} ${jqScript} < ${json}
						${jqPrefix}${jq} ${jqScript} < ${json} > ${tg.output}/output.json
					`,
					{ target },
				);
			"#,
		);

		// Extra indentation should be removed from multi-line templates that are indented too far with interpolation.
		assert_eq_after_reindent!(
			r#"
				let jqPrefix = "";
				let json = tg.file('{"foo": "bar"}');
				let jqScript = "'.foo'";
				std.bash(
					t`
							mkdir ${tg.output}
							${jqPrefix}${jq} ${jqScript} < ${json}
							${jqPrefix}${jq} ${jqScript} < ${json} > ${tg.output}/output.json
					`,
					{ target },
				);
			"#,
			r#"
				let jqPrefix = "";
				let json = tg.file('{"foo": "bar"}');
				let jqScript = "'.foo'";
				std.bash(
					t`
						mkdir ${tg.output}
						${jqPrefix}${jq} ${jqScript} < ${json}
						${jqPrefix}${jq} ${jqScript} < ${json} > ${tg.output}/output.json
					`,
					{ target },
				);
			"#,
		);
	}

	#[test]
	fn test_reindent_with_inner_indentation() {
		// When there's too much indentation, it should be un-indented, but extra indentation beyond the baseline should be preserved.
		assert_eq_after_reindent!(
			r#"
				std.bash(
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
				std.bash(
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
		assert_eq_after_reindent!(
			r#"
				std.bash(
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
				std.bash(
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

	#[test]
	#[allow(clippy::too_many_lines)]
	fn test_reindent_starts_and_ends_with_a_blank_line() {
		// For a multi-line template, a newline should be added to the start so the first line of the template starts on its own line.
		assert_eq_after_reindent!(
			r#"
				std.bash(
					t`echo "hello" > ${tg.output}
						echo "world" >> file.txt
					`,
					{ target },
				);
			"#,
			r#"
				std.bash(
					t`
						echo "hello" > ${tg.output}
						echo "world" >> file.txt
					`,
					{ target },
				);
			"#,
		);

		// For a multi-line template, a newline should be added to the end so the closing backtick is on its own line.
		assert_eq_after_reindent!(
			r#"
				std.bash(
					t`
						echo "hello" > ${tg.output}
						echo "world" >> file.txt`,
					{ target },
				);
			"#,
			r#"
				std.bash(
					t`
						echo "hello" > ${tg.output}
						echo "world" >> file.txt
					`,
					{ target },
				);
			"#,
		);

		// We may need to add a newline both to the start and the end.
		assert_eq_after_reindent!(
			r#"
				std.bash(
					t`echo "hello" > ${tg.output}
						echo "world" >> file.txt`,
					{ target },
				);
			"#,
			r#"
				std.bash(
					t`
						echo "hello" > ${tg.output}
						echo "world" >> file.txt
					`,
					{ target },
				);
			"#,
		);

		// A newline should be added even if the template starts with interpolation. This is a special case because the first element of the template is a different node here.
		assert_eq_after_reindent!(
			r#"
				let echo = "echo";
				std.bash(
					t`${echo} "hello" > ${tg.output}
						echo "world" >> file.txt`,
					{ target },
				);
			"#,
			r#"
				let echo = "echo";
				std.bash(
					t`
						${echo} "hello" > ${tg.output}
						echo "world" >> file.txt
					`,
					{ target },
				);
			"#,
		);

		// A newline should be added even if the template ends with interpolation. This is a special case because the last element of the template is a different node here.
		assert_eq_after_reindent!(
			r#"
				std.bash(
					t`echo "hello" > ${tg.output}
						echo "world" >> ${tg.output}`,
					{ target },
				);
			"#,
			r#"
				std.bash(
					t`
						echo "hello" > ${tg.output}
						echo "world" >> ${tg.output}
					`,
					{ target },
				);
			"#,
		);

		// A newline should be added even if the template starts _and_ ends with interpolation. Here, we need to add a new node to both the start and end of the template expression.
		assert_eq_after_reindent!(
			r#"
				let echo = "echo";
				std.bash(
					t`${echo} "hello" > ${tg.output}
						echo "world" >> ${tg.output}`,
					{ target },
				);
			"#,
			r#"
				let echo = "echo";
				std.bash(
					t`
						${echo} "hello" > ${tg.output}
						echo "world" >> ${tg.output}
					`,
					{ target },
				);
			"#,
		);

		assert_eq_after_reindent!(
			r#"
				import * as std from "tangram:std";

				type Args = {
					target: tg.System;
				};

				export let foo = tg.createTarget(({ target }: Args) => {
					return std.bash(
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

				export let foo = tg.createTarget(({ target }: Args) => {
					return std.bash(
						t`
							echo Hello world

						`,
						{ system: target },
					);
				});
			"#,
		);
	}
}
