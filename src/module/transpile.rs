use crate::{
	error::{Error, Location, Result},
	module::Module,
};
use std::rc::Rc;
use swc_common::{Globals, Mark, SourceMap, Span, GLOBALS};
use swc_ecma_ast::{
	CallExpr, EsVersion, ExportDecl, ExportDefaultExpr, Expr, ExprOrSpread, Ident, KeyValueProp,
	Lit, ObjectLit, Prop, PropOrSpread, Str, TsEntityName, TsQualifiedName, TsType,
	TsTypeParamInstantiation, TsTypeRef, VarDeclarator,
};
use swc_ecma_codegen::{text_writer::JsWriter, Config, Emitter, Node};
use swc_ecma_transforms::{fixer, resolver, typescript::strip};
use swc_ecma_visit::{FoldWith, VisitMut, VisitMutWith};

#[derive(Debug)]
pub struct Output {
	pub transpiled_text: String,
	pub source_map: String,
}

impl Module {
	pub fn transpile(text: String) -> Result<Output> {
		// Parse the module.
		let (module, source_map) = Module::parse(text)?;

		// Execute the resolver, strip types, and emit JS.
		let globals = Globals::default();
		GLOBALS.set(&globals, move || {
			let unresolved_mark = Mark::new();
			let top_level_mark = Mark::new();

			let module = module.fold_with(&mut resolver(unresolved_mark, top_level_mark, true));

			// TODO: Apply transforms.
			// Module::expand_transforms(&mut module, source_map.clone())?;

			// Strip types and run the "fixer" pass. Note: stripping types emits invalid JS that the fixer pass is required to touchup.
			let module = module.fold_with(&mut strip(top_level_mark));
			let module = module.fold_with(&mut fixer(None));
			Module::emit(&module, &source_map)
		})
	}

	pub fn emit(module: &swc_ecma_ast::Module, source_map: &Rc<SourceMap>) -> Result<Output> {
		// Emit the output.
		let mut output_text = Vec::new();
		let mut source_mappings = Vec::new();
		let writer = Box::new(JsWriter::new(
			source_map.clone(),
			"\n",
			&mut output_text,
			Some(&mut source_mappings),
		));

		let config = Config {
			minify: false,
			ascii_only: false,
			omit_last_semi: false,
			target: EsVersion::EsNext,
		};

		let mut emitter = Emitter {
			cfg: config,
			comments: None,
			cm: source_map.clone(),
			wr: writer,
		};

		module.emit_with(&mut emitter).map_err(Error::other)?;

		// Convert the source mappings to source map text.
		let mut output_source_map = Vec::new();
		source_map
			.build_source_map(&source_mappings)
			.to_writer(&mut output_source_map)
			.map_err(Error::other)?;

		// Convert the output buffers to Strings.
		let transpiled_text = String::from_utf8(output_text).map_err(Error::other)?;
		let source_map = String::from_utf8(output_source_map).map_err(Error::other)?;

		Ok(Output {
			transpiled_text,
			source_map,
		})
	}

	pub fn expand_transforms(
		module: &mut swc_ecma_ast::Module,
		source_map: &Rc<SourceMap>,
	) -> Result<()> {
		let mut expand_tg_function = ExpandTgFunction {
			source_map: source_map.clone(),
			errors: Vec::new(),
		};

		let mut expand_tg_include = ExpandTgInclude {
			source_map: source_map.clone(),
			errors: Vec::new(),
		};

		module.visit_mut_with(&mut expand_tg_function);
		module.visit_mut_with(&mut expand_tg_include);

		let mut errors = expand_tg_function.errors;
		errors.extend(expand_tg_include.errors);

		if !errors.is_empty() {
			let message = errors
				.into_iter()
				.map(|e| e.to_string())
				.collect::<Vec<_>>()
				.join("\n");
			return Err(Error::message(message));
		}
		Ok(())
	}
}

// Macro-expansion of tg.function calls.
struct ExpandTgFunction {
	source_map: Rc<SourceMap>,
	errors: Vec<Error>,
}

impl ExpandTgFunction {
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

	fn add_object_argument_to_call(&mut self, expr: &mut CallExpr, name: &str, span: Span) {
		// Check if we're visiting a tg.function() call.
		let Some(callee) = expr.callee.as_expr().and_then(|e| e.as_member()) else { return };
		let Some(obj) = callee.obj.as_ident() else { return };
		let Some(prop) = callee.prop.as_ident() else { return };

		if (&obj.sym) != "tg" || (&prop.sym) != "function" {
			return;
		}

		// Validate the arguments to the call.
		if expr.args.len() != 1 {
			self.add_error("Invalid number of arguments to tg.function.", expr.span);
			return;
		}

		// Create an object, { name: <name> }.
		let key = Ident::new("name".into(), span);
		let value: Expr = Lit::Str(Str {
			value: name.into(),
			span,
			raw: None,
		})
		.into();
		let prop = Prop::KeyValue(KeyValueProp {
			key: key.into(),
			value: Box::new(value),
		});
		let object = ObjectLit {
			props: vec![PropOrSpread::Prop(Box::new(prop))],
			span,
		};

		// Add the object to the arguments.
		expr.args.push(ExprOrSpread {
			spread: None,
			expr: object.into(),
		});
	}
}

impl VisitMut for ExpandTgFunction {
	fn visit_mut_export_default_expr(&mut self, n: &mut ExportDefaultExpr) {
		// Check that this is a function call expression.
		let Some(expr) = n.expr.as_mut_call() else { return };

		// Attempt to add { name: <name> } to a tg.function invocation.
		self.add_object_argument_to_call(expr, "default", n.span);

		// Continue visiting children.
		n.visit_mut_children_with(self);
	}

	fn visit_mut_export_decl(&mut self, n: &mut ExportDecl) {
		// Check that this is an expression of the form "export let <name> = <function call>"
		let Some(decl) = n.decl.as_mut_var() else { return; };
		if decl.decls.len() != 1 {
			return;
		}

		let VarDeclarator {
			name, init, span, ..
		} = &mut decl.decls[0];
		let Some(name) = name.as_ident().map(|i| &i.id) else { return };
		let Some(init) = init.as_deref_mut() else { return };
		let Some(expr) = init.as_mut_call() else { return };

		// Attempt to add { name: <name> } to a tg.function invocation.
		self.add_object_argument_to_call(expr, name.as_ref(), *span);

		// Continue visiting children.
		n.visit_mut_children_with(self);
	}
}

// Macro-expansion of tg.include calls.
struct ExpandTgInclude {
	source_map: Rc<SourceMap>,
	errors: Vec<Error>,
}

impl ExpandTgInclude {
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
}

impl VisitMut for ExpandTgInclude {
	fn visit_mut_call_expr(&mut self, n: &mut CallExpr) {
		let span = n.span;

		// Check if we're visiting a tg.include() call.
		let Some(callee) = n.callee.as_expr().and_then(|e| e.as_member()) else { return };
		let Some(obj) = callee.obj.as_ident() else { return };
		let Some(prop) = callee.prop.as_ident() else { return };

		if (&obj.sym) != "tg" || (&prop.sym) != "include" {
			return;
		}

		// Validate the arguments to the call.
		if n.args.len() != 1 {
			self.add_error("Invalid number of arguments to tg.include.", n.span);
			return;
		}
		let Some(Lit::Str(arg)) = n.args[0].expr.as_lit() else {
			self.add_error("tg.include() must be called with a string literal as an argument.", n.span);
			return;
		};

		// Extract the type of the file.
		let path = arg.value.as_ref();
		let type_ = match std::fs::metadata(path) {
			Ok(stat) if stat.is_symlink() => "Symlink",
			Ok(stat) if stat.is_dir() => "Directory",
			Ok(_) => "File",
			Err(e) => {
				self.add_error(&e.to_string(), arg.span);
				return;
			},
		};

		// Add a type parameter to the expression.
		let qualified_name = TsQualifiedName {
			left: Ident::new("tg".into(), span).into(),
			right: Ident::new(type_.into(), span),
		};

		let type_params = TsTypeParamInstantiation {
			span,
			params: vec![Box::new(TsType::TsTypeRef(TsTypeRef {
				span,
				type_name: TsEntityName::TsQualifiedName(Box::new(qualified_name)),
				type_params: None,
			}))],
		};

		n.type_args = Some(Box::new(type_params));
		n.visit_mut_children_with(self);
	}
}

#[cfg(test)]
mod tests {
	use super::Output;
	use crate::module::Module;

	#[test]
	fn export_default_function() {
		let text = r#"export default tg.function(arg)"#;
		let Output {
			transpiled_text, ..
		} = Module::transpile(text.to_owned()).expect("Failed to transpile text.");
		assert_eq!(
			transpiled_text,
			"export default tg.function(arg, {\n    name: \"default\"\n});\n"
		);
	}

	#[test]
	fn export_named_function() {
		let text = r#"export let named = tg.function(arg)"#;
		let Output {
			transpiled_text, ..
		} = Module::transpile(text.to_owned()).expect("Failed to transpile text.");
		assert_eq!(
			transpiled_text,
			"export let named = tg.function(arg, {\n    name: \"named\"\n});\n"
		);
	}

	#[test]
	fn hello() {
		let text = r#"
			import * as std from "tangram:std";
			export let named = tg.function(() => "hello");
			export default tg.function(() => {
				return named();
			});
		"#;
		let Output {
			transpiled_text, ..
		} = Module::transpile(text.to_owned()).expect("Failed to transpile the text.");
		eprintln!("{transpiled_text}");
	}
}
