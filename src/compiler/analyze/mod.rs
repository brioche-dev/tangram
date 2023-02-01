use anyhow::Result;
use rome_analyze::{ControlFlow, SuppressionCommentEmitterPayload, SuppressionKind};
use rome_js_syntax::{suppression::SuppressionDiagnostic, AnyJsRoot, JsLanguage};
use rome_rowan::AstNode;

mod rules;

/// The number of times to repeatedly apply the fixes from rules before giving up. If it takes longer than this to fix all the rules, we assume that there's a problem with one or more rules.
const MAX_FIX_ITERATIONS: u32 = 1000;

/// Apply all linting fixes to the given JavaScript/TypeScript AST. The filter specifies which rules will apply, which can include any default Rome rules and any Tangram-specific rules.
pub fn fix_all(ast_root: &mut AnyJsRoot, rule_filter: rome_analyze::AnalysisFilter) -> Result<()> {
	// Repeatedly apply fixes until there are no more fixes to apply.
	for n in 0.. {
		// If we've iterated too many times, this can often be a sign of a bug in a rule. This is a safeguard that helps when writing new rules.
		anyhow::ensure!(
			n <= MAX_FIX_ITERATIONS,
			"Applying fixes did not terminate after {MAX_FIX_ITERATIONS} iterations. This likely means a rule returned an action that did not resolve the rule or that two rules conflict."
		);

		// Run the analyzer.
		let action = analyze(ast_root, rule_filter, |signal| {
			for action in signal.actions() {
				// Ignore suppression actions.
				if action.is_suppression() {
					continue;
				}
				if matches!(
					action.applicability,
					rome_diagnostics::Applicability::Always
						| rome_diagnostics::Applicability::MaybeIncorrect
				) {
					// This rule applies, so stop and handle it.
					return ControlFlow::Break(action);
				}
			}

			// No actions applied, so look for the next signal.
			ControlFlow::Continue(())
		});

		match action {
			Some(action) => {
				// If we broke out with an action, update the AST based on the edits from the action. This will effectively fix the linting rule.
				if let Some((_, _)) = action.mutation.as_text_edits() {
					*ast_root = match AnyJsRoot::cast(action.mutation.commit()) {
						Some(tree) => tree,
						None => {
							anyhow::bail!("Rule tried to replace root with non-root node.");
						},
					};
				}
			},
			None => {
				// Otherwise, we reached the end of all rules without finding any actions to apply. We're done.
				break;
			},
		}
	}

	Ok(())
}

/// Run Rome's analyzer against the given AST and filtered to the desired set of rules. `emit_signal` is passed directly to Rome's analyzer and will be called for each signal emitted by the analyzer. This code is very similar to how Rome handles linting.
fn analyze<'a, F, B>(
	ast_root: &AnyJsRoot,
	rule_filter: rome_analyze::AnalysisFilter,
	mut emit_signal: F,
) -> Option<B>
where
	F: FnMut(&dyn rome_analyze::AnalyzerSignal<JsLanguage>) -> ControlFlow<B> + 'a,
{
	// Use the default analyzer options.
	let analyzer_options = rome_analyze::AnalyzerOptions::default();

	// Build a registry of rules.
	let mut rule_registry = rome_analyze::RuleRegistry::<JsLanguage>::builder(
		&rule_filter,
		&analyzer_options,
		ast_root,
	);

	// Add Rome's default rules.
	rome_js_analyze::visit_registry(&mut rule_registry);

	// Add Tangram custom rules.
	rules::visit_registry(&mut rule_registry);

	// Build the registry.
	let (registry, services, _diagnostics, visitors) = rule_registry.build();

	// Create the analyzer. For now, we pass in dummy functions where needed (`inspect_matcher`, `parse_linter_suppression_comment`, `apply_suppression_comment`).
	let inspect_matcher = rome_analyze::InspectMatcher::new(registry, |_| {});
	let mut analyzer = rome_analyze::Analyzer::new(
		rome_js_analyze::metadata(),
		inspect_matcher,
		parse_linter_suppression_comment,
		apply_suppression_comment,
		&mut emit_signal,
	);

	// Add each visitor to the analyzer.
	for ((phase, _), visitor) in visitors {
		analyzer.add_visitor(phase, visitor);
	}

	// Run the analyzer.
	analyzer.run(rome_analyze::AnalyzerContext {
		root: ast_root.clone(),
		range: rule_filter.range,
		services,
		options: &analyzer_options,
	})
}

fn parse_linter_suppression_comment(
	_text: &str,
) -> Vec<Result<SuppressionKind, SuppressionDiagnostic>> {
	vec![]
}

#[allow(clippy::needless_pass_by_value)]
fn apply_suppression_comment(_payload: SuppressionCommentEmitterPayload<JsLanguage>) {}
