use super::parse;
use crate::{
	error::{Error, Location, Result, WrapErr},
	module::{self, Module},
	path::Relpath,
};
use std::{collections::HashSet, rc::Rc, str::FromStr};
use swc_common::{SourceMap, Span, DUMMY_SP};
use swc_ecma_ast::{CallExpr, Callee, ExportAll, Expr, Ident, ImportDecl, Lit, NamedExport};
use swc_ecma_visit::{Visit, VisitWith};

#[derive(Clone, Debug)]
pub struct Output {
	pub imports: Vec<module::Import>,
	pub includes: Vec<Relpath>,
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
			return Err(Error::message(message));
		}

		// Collect the imports and includes.
		let imports = visitor.imports.into_iter().map(Into::into).collect();
		let includes = visitor.includes.into_iter().map(Into::into).collect();

		// Create the output.
		let output = Output { imports, includes };

		Ok(output)
	}
}

#[derive(Default)]
struct Visitor {
	source_map: Rc<SourceMap>,
	imports: HashSet<module::Import, fnv::FnvBuildHasher>,
	includes: HashSet<Relpath, fnv::FnvBuildHasher>,
	errors: Vec<Error>,
}

impl Visitor {
	fn new(source_map: Rc<SourceMap>) -> Self {
		Self {
			source_map,
			..Default::default()
		}
	}

	fn add_error(&mut self, message: &str, span: Span) {
		let start = self.source_map.lookup_char_pos(span.lo);
		let location = Location {
			file: start.file.name.to_string(),
			line: start.line.try_into().unwrap(),
			column: start.col_display.try_into().unwrap(),
		};
		let error = Error::Message {
			message: message.to_owned(),
			location,
			source: None,
		};
		self.errors.push(error);
	}

	fn add_import(&mut self, specifier: &str, span: Span) {
		let specifier = module::Import::from_str(specifier);
		match specifier {
			Ok(specifier) => {
				self.imports.insert(specifier);
			},
			Err(e) => {
				self.add_error(&e.to_string(), span);
			},
		}
	}
}

impl Visit for Visitor {
	fn visit_import_decl(&mut self, n: &ImportDecl) {
		self.add_import(&n.src.value, n.span);
		n.visit_children_with(self);
	}

	fn visit_named_export(&mut self, n: &NamedExport) {
		if let Some(src) = n.src.as_deref() {
			self.add_import(&src.value, n.span);
		}
		n.visit_children_with(self);
	}

	fn visit_export_all(&mut self, n: &ExportAll) {
		self.add_import(&n.src.value, n.span);
		n.visit_children_with(self);
	}

	fn visit_expr(&mut self, n: &Expr) {
		if let Some(expr) = n.as_call() {
			match &expr.callee {
				Callee::Expr(callee) => self.visit_call(expr, callee),
				Callee::Import(_) => self.visit_import(expr),
				Callee::Super(_) => (),
			}
		}
		n.visit_children_with(self);
	}
}

impl Visitor {
	fn visit_call(&mut self, expr: &CallExpr, callee: &Expr) {
		let Some(callee) = callee.as_member() else { return };
		let tg = Ident::new("tg".into(), DUMMY_SP).to_id().0;
		let include = Ident::new("include".into(), DUMMY_SP).to_id().0;
		if !(callee.obj.as_ident().map(|id| id.to_id().0) == Some(tg)
			&& callee.prop.as_ident().map(|id| id.to_id().0) == Some(include))
		{
			return;
		}

		// Validate the arguments and add to the includes list.
		if expr.args.len() != 1 {
			self.add_error("Invalid number of arguments to tg.include().", expr.span);
			return;
		}

		if let Some(Lit::Str(argument)) = expr.args[0].expr.as_lit() {
			let Ok(include) = argument.value.to_string().parse() else {
				self.add_error("Failed to parse the include.", expr.span);
				return;
			};
			self.includes.insert(include);
		} else {
			self.add_error(
				"tg.include() may only take a string literal as an argument.",
				expr.span,
			);
		}
	}

	fn visit_import(&mut self, expr: &CallExpr) {
		let Some(arg) = expr.args.first() else {
			self.add_error("Invalid number of arguments to import().", expr.span);
			return;
		};
		let Some(Lit::Str(arg)) = arg.expr.as_lit() else {
			self.add_error(
				"import() may only take a string literal as an argument.",
				expr.span,
			);
			return;
		};
		self.add_import(&arg.value, expr.span);
	}
}

#[cfg(test)]
mod tests {
	use crate::module::Module;

	#[test]
	fn analyze() {
		let text = r#"
			import * as glob from "tangram:glob";
			import { named } from "tangram:named";
			import mod from "./module.tg";

			let dynamic = import("./dynamic.tg");
			let included = tg.include("./included.txt");

			export let nested = tg.function(() => {
					let nestedDynamic = import("tangram:nested_dynamic");
					let nestedInclude = tg.include("./nestedInclude.txt");
			});

			export * as rexxport from "./reexport.tg";
			export { thing } from "tangram:named_export";
		"#;
		let output = Module::analyze(text.to_owned()).expect("Failed to analyze the source code.");
		let import = "tangram:glob".parse().unwrap();
		assert!(output.imports.contains(&import));
		let import = "tangram:named".parse().unwrap();
		assert!(output.imports.contains(&import));
		let import = "./module.tg".parse().unwrap();
		assert!(output.imports.contains(&import));
		let import = "./dynamic.tg".parse().unwrap();
		assert!(output.imports.contains(&import));
		let include = "./included.txt".parse().unwrap();
		assert!(output.includes.contains(&include));
		let import = "tangram:nested_dynamic".parse().unwrap();
		assert!(output.imports.contains(&import));
		let include = "./nestedInclude.txt".parse().unwrap();
		assert!(output.includes.contains(&include));
		let import = "./reexport.tg".parse().unwrap();
		assert!(output.imports.contains(&import));
		let import = "tangram:named_export".parse().unwrap();
		assert!(output.imports.contains(&import));
	}

	#[test]
	fn invalid_tg_include() {
		let text = r#"tg.include("foo", bar)"#;
		let result = Module::analyze(text.to_owned());
		assert!(result.is_err());

		let text = r#"tg.include(bar)"#;
		let result = Module::analyze(text.to_owned());
		assert!(result.is_err());
	}

	#[test]
	fn invalid_import() {
		let text = r#"import("./foo.tg", bar)"#;
		let result = Module::analyze(text.to_owned());
		assert!(result.is_err());

		let text = r#"import(bar)"#;
		let result = Module::analyze(text.to_owned());
		assert!(result.is_err());

		let text = r#"import * as foo from "foo""#;
		let result = Module::analyze(text.to_owned());
		assert!(result.is_err());
	}
}
