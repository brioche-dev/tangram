use super::{error::Error, parse, Import};
use crate::{
	error::{Result, WrapErr},
	module::Module,
	package::Metadata,
	path::Relpath,
};
use std::{collections::HashSet, rc::Rc};
use swc_core::{
	common::{SourceMap, Span},
	ecma::{
		ast::{CallExpr, Callee, ExportAll, ImportDecl, Lit, NamedExport},
		visit::{Visit, VisitWith},
	},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
	pub metadata: Option<Metadata>,
	pub imports: HashSet<Import, fnv::FnvBuildHasher>,
	pub includes: HashSet<Relpath, fnv::FnvBuildHasher>,
}

impl Module {
	#[tracing::instrument(skip(text))]
	pub fn analyze(text: String) -> Result<Output> {
		// Parse the text.
		let parse::Output { module, source_map } =
			Module::parse(text).wrap_err("Failed to parse the module.")?;

		// Create the visitor and visit the module.
		let mut visitor = Visitor::new(source_map);
		module.visit_with(&mut visitor);

		// Handle any errors.
		let errors = visitor.errors;
		if !errors.is_empty() {
			let message = errors
				.iter()
				.map(std::string::ToString::to_string)
				.collect::<Vec<_>>()
				.join("\n");
			return Err(crate::error::Error::message(message));
		}

		// Create the output.
		let output = Output {
			metadata: None,
			imports: visitor.imports,
			includes: visitor.includes,
		};

		Ok(output)
	}
}

#[derive(Default)]
struct Visitor {
	source_map: Rc<SourceMap>,
	errors: Vec<Error>,
	imports: HashSet<Import, fnv::FnvBuildHasher>,
	includes: HashSet<Relpath, fnv::FnvBuildHasher>,
}

impl Visitor {
	fn new(source_map: Rc<SourceMap>) -> Self {
		Self {
			source_map,
			..Default::default()
		}
	}
}

impl Visit for Visitor {
	fn visit_import_decl(&mut self, n: &ImportDecl) {
		self.add_import(&n.src.value, n.span);
	}

	fn visit_named_export(&mut self, n: &NamedExport) {
		if let Some(src) = n.src.as_deref() {
			self.add_import(&src.value, n.span);
		}
	}

	fn visit_export_all(&mut self, n: &ExportAll) {
		self.add_import(&n.src.value, n.span);
	}

	fn visit_call_expr(&mut self, n: &CallExpr) {
		match &n.callee {
			// Handle a call expression.
			Callee::Expr(callee) => {
				// Ignore call expressions that are not tg.include().
				let Some(callee) = callee.as_member() else {
					n.visit_children_with(self);
					return;
				};
				let Some(obj) = callee.obj.as_ident() else {
					n.visit_children_with(self);
					return;
				};
				let Some(prop) = callee.prop.as_ident() else {
					n.visit_children_with(self);
					return;
				};
				if !(&obj.sym == "tg" && &prop.sym == "include") {
					n.visit_children_with(self);
					return;
				}

				// Get the location of the call.
				let loc = self.source_map.lookup_char_pos(n.span.lo);

				// Get the argument and verify it is a string literal.
				if n.args.len() != 1 {
					self.errors.push(Error::new(
						"tg.include must be called with exactly one argument.",
						&loc,
					));
					return;
				}
				let Some(Lit::Str(arg)) = n.args[0].expr.as_lit() else {
					self.errors.push(Error::new("The argument to tg.include must be a string literal.", &loc));
					return;
				};

				// Parse the argument and add it to the set of includes.
				let Ok(include) = arg.value.to_string().parse() else {
					self.errors.push(Error::new("Failed to parse the argument to tg.include.", &loc));
					return;
				};
				self.includes.insert(include);
			},

			// Handle a dynamic import.
			Callee::Import(_) => {
				let Some(Lit::Str(arg)) = n.args.first().and_then(|arg| arg.expr.as_lit()) else {
					let loc = self.source_map.lookup_char_pos(n.span.lo);
					self.errors.push(Error::new(
						"The argument to the import function must be a string literal.",
						&loc,
					));
					return;
				};
				self.add_import(&arg.value, n.span);
			},

			// Ignore a call to super.
			Callee::Super(_) => {
				n.visit_children_with(self);
			},
		}
	}
}

impl Visitor {
	fn add_import(&mut self, import: &str, span: Span) {
		let Ok(import) = import.parse() else {
			let loc = self.source_map.lookup_char_pos(span.lo());
			self.errors
				.push(Error::new("Failed to parse the import.", &loc));
			return;
		};
		self.imports.insert(import);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_analyze() {
		let text = r#"
			export let metadata = { name: "name", version: "version" };
			import defaultImport from "tangram:default_import";
			import { namedImport } from "./named_import.tg";
			import * as namespaceImport from "tangram:namespace_import";
			let dynamicImport = import("./dynamic_import.tg");
			let include = tg.include("./include.txt");
			export let nested = tg.function(() => {
				let nestedDynamicImport = import("tangram:nested_dynamic_import");
				let nestedInclude = tg.include("./nested_include.txt");
			});
			export { namedExport } from "tangram:named_export";
			export * as namespaceExport from "./namespace_export.tg";
		"#;
		let left = Module::analyze(text.to_owned()).unwrap();
		let metadata = Metadata {
			name: Some("name".to_owned()),
			version: Some("version".to_owned()),
		};
		let imports = [
			"tangram:default_import",
			"./named_import.tg",
			"tangram:namespace_import",
			"./dynamic_import.tg",
			"tangram:nested_dynamic_import",
			"tangram:named_export",
			"./namespace_export.tg",
		]
		.into_iter()
		.map(|import| import.parse().unwrap())
		.collect();
		let includes = ["./include.txt", "./nested_include.txt"]
			.into_iter()
			.map(|include| include.parse().unwrap())
			.collect();
		let right = Output {
			metadata: Some(metadata),
			imports,
			includes,
		};
		assert_eq!(left, right);
	}
}
