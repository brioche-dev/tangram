use crate::{
	error::{Error, Location, Result},
	module::{self, Module},
	path::Relpath,
};
use std::{collections::HashSet, rc::Rc, str::FromStr};
use swc_common::{SourceMap, Span, DUMMY_SP};
use swc_ecma_ast::{CallExpr, Callee, ExportAll, Expr, Ident, ImportDecl, Lit, NamedExport};
use swc_ecma_visit::{Visit, VisitWith};

#[derive(Clone, Debug)]
pub struct Output {
	pub imports: Vec<module::Specifier>,
	pub includes: Vec<Relpath>,
}

impl Module {
	#[tracing::instrument(skip(text))]
	pub fn analyze(text: String) -> Result<Output> {
		// Parse the input.
		let (module, source_map) = Module::parse(text)?;

		// Construct our visitor and walk the AST.
		let mut visitor = Visitor {
			source_map,
			..Default::default()
		};
		module.visit_with(&mut visitor);

		// Convert into paths and specifiers.
		let imports = visitor.imports.into_iter().map(Into::into).collect();
		let includes = visitor.includes.into_iter().map(Into::into).collect();
		let errors = visitor.errors;

		if !errors.is_empty() {
			// TODO: better error reporting.
			let message = errors
				.iter()
				.map(std::string::ToString::to_string)
				.collect::<Vec<_>>()
				.join("\n");
			return Err(Error::message(message));
		}
		Ok(Output { imports, includes })
	}
}

// The visitor context.
#[derive(Default)]
struct Visitor {
	source_map: Rc<SourceMap>,
	imports: HashSet<module::Specifier, fnv::FnvBuildHasher>,
	includes: HashSet<Relpath, fnv::FnvBuildHasher>,
	errors: Vec<Error>,
}

impl Visit for Visitor {
	// import <xxx> from <source>
	fn visit_import_decl(&mut self, n: &ImportDecl) {
		self.add_import(&n.src.value, n.span);
		n.visit_children_with(self);
	}

	// export { <named> } from <source>
	fn visit_named_export(&mut self, n: &NamedExport) {
		if let Some(src) = n.src.as_deref() {
			self.add_import(&src.value, n.span);
		}
		n.visit_children_with(self);
	}

	// export * from <source>
	fn visit_export_all(&mut self, n: &ExportAll) {
		self.add_import(&n.src.value, n.span);
		n.visit_children_with(self);
	}

	// tg.include(<source>), import(<source>)
	fn visit_expr(&mut self, n: &Expr) {
		if let Some(expr) = n.as_call() {
			match &expr.callee {
				Callee::Expr(callee) => self.visit_call(expr, callee),
				Callee::Import(_) => self.visit_import_expr(expr),
				Callee::Super(_) => (),
			}
		}
		n.visit_children_with(self);
	}
}

impl Visitor {
	// Add an import specifier to the context.
	fn add_import(&mut self, specifier: &str, span: Span) {
		// TODO: Use a real error when import specifiers can't be parsed.
		let specifier = module::Specifier::from_str(specifier);
		match specifier {
			Ok(specifier) => {
				self.imports.insert(specifier);
			},
			Err(e) => {
				self.add_error(&e.to_string(), span);
			},
		}
	}

	// Add an error message to the context.
	fn add_error(&mut self, message: &str, span: Span) {
		tracing::error!(?message);
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

	// tg.include(<source>)
	fn visit_call(&mut self, expr: &CallExpr, callee: &Expr) {
		// Check if the callee is a member of some object.
		let Some(callee) = callee.as_member() else { return };

		// Check if it's a call to tg.include.
		let tg = Ident::new("tg".into(), DUMMY_SP).to_id().0;
		let include = Ident::new("include".into(), DUMMY_SP).to_id().0;

		let is_tg_include = callee.obj.as_ident().map(|id| id.to_id().0) == Some(tg)
			&& callee.prop.as_ident().map(|id| id.to_id().0) == Some(include);

		if !is_tg_include {
			return;
		}

		// Validate the arguments and add to the includes list.
		if expr.args.len() != 1 {
			self.add_error("Invalid number of arguments to tg.include().", expr.span);
			return;
		}

		let argument = &expr.args[0].expr.as_lit();
		if let Some(Lit::Str(argument)) = argument {
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

	// import(<source>)
	fn visit_import_expr(&mut self, expr: &CallExpr) {
		if expr.args.len() != 1 {
			self.add_error("Invalid number of arguments to import().", expr.span);
			return;
		}

		let argument = expr.args[0].expr.as_lit();
		if let Some(Lit::Str(argument)) = argument {
			self.add_import(&argument.value, expr.span);
		} else {
			self.add_error(
				"import() may only take a string literal as an argument.",
				expr.span,
			);
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::module::{self, Module};

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

		let spec = module::Specifier::try_from("tangram:glob".to_owned()).unwrap();
		assert!(output.imports.contains(&spec));
		let spec = module::Specifier::try_from("tangram:named".to_owned()).unwrap();
		assert!(output.imports.contains(&spec));
		let spec = module::Specifier::try_from("./module.tg".to_owned()).unwrap();
		assert!(output.imports.contains(&spec));
		let spec = module::Specifier::try_from("./dynamic.tg".to_owned()).unwrap();
		assert!(output.imports.contains(&spec));
		let spec = module::Specifier::try_from("tangram:nested_dynamic".to_owned()).unwrap();
		assert!(output.imports.contains(&spec));
		let spec = module::Specifier::try_from("./reexport.tg".to_owned()).unwrap();
		assert!(output.imports.contains(&spec));
		let spec = module::Specifier::try_from("./reexport.tg".to_owned()).unwrap();
		assert!(output.imports.contains(&spec));
		let spec = module::Specifier::try_from("tangram:named_export".to_owned()).unwrap();
		assert!(output.imports.contains(&spec));

		let path = "./included.txt".parse().unwrap();
		assert!(output.includes.contains(&path));
		let path = "./nestedInclude.txt".parse().unwrap();
		assert!(output.includes.contains(&path));
	}

	#[test]
	fn invalid_tg_include() {
		let text = r#"tg.include("foo", bar)"#;
		let output = Module::analyze(text.to_owned());
		assert!(output.is_err());
		eprintln!("{}", output.unwrap_err());

		let text = r#"tg.include(bar)"#;
		let output = Module::analyze(text.to_owned());
		assert!(output.is_err());
		eprintln!("{}", output.unwrap_err());
	}

	#[test]
	fn invalid_import() {
		let text = r#"import("./foo.tg", bar)"#;
		let output = Module::analyze(text.to_owned());
		assert!(output.is_err());
		eprintln!("{}", output.unwrap_err());

		let text = r#"import(bar)"#;
		let output = Module::analyze(text.to_owned());
		assert!(output.is_err());
		eprintln!("{}", output.unwrap_err());

		let text = r#"import * as foo from "foo""#;
		let output = Module::analyze(text.to_owned());
		assert!(output.is_err());
		eprintln!("{}", output.unwrap_err());
	}
}
