use super::{error::Error, parse};
use crate::{
	error::{Result, WrapErr},
	module::Module,
};
use std::rc::Rc;
use swc_core::{
	common::{Globals, Mark, SourceMap, DUMMY_SP, GLOBALS},
	ecma::{
		ast::{
			CallExpr, ExportDecl, ExportDefaultExpr, Expr, ExprOrSpread, Ident, KeyValueProp, Lit,
			MemberExpr, MetaPropExpr, ObjectLit, Prop, PropOrSpread, Str, VarDeclarator,
		},
		codegen::{text_writer::JsWriter, Config, Emitter},
		transforms::{
			base::{fixer::fixer, resolver},
			typescript::strip,
		},
		visit::{VisitMut, VisitMutWith},
	},
};

#[derive(Debug)]
pub struct Output {
	pub transpiled_text: String,
	pub source_map: String,
}

impl Module {
	pub fn transpile(text: String) -> Result<Output> {
		let globals = Globals::default();
		GLOBALS.set(&globals, move || {
			// Parse the text.
			let parse::Output {
				mut module,
				source_map,
			} = Module::parse(text)?;

			// Create the target visitor.
			let mut target_visitor = TargetVisitor {
				source_map: source_map.clone(),
				errors: Vec::new(),
			};

			// Create the include visitor.
			let mut include_visitor = IncludeVisitor {
				source_map: source_map.clone(),
				errors: Vec::new(),
			};

			// Visit the module.
			let unresolved_mark = Mark::new();
			let top_level_mark = Mark::new();
			module.visit_mut_with(&mut resolver(unresolved_mark, top_level_mark, true));
			module.visit_mut_with(&mut target_visitor);
			module.visit_mut_with(&mut include_visitor);
			module.visit_mut_with(&mut strip(top_level_mark));
			module.visit_mut_with(&mut fixer(None));

			// Create the writer.
			let mut transpiled_text = Vec::new();
			let mut source_mappings = Vec::new();
			let mut writer = JsWriter::new(
				source_map.clone(),
				"\n",
				&mut transpiled_text,
				Some(&mut source_mappings),
			);
			writer.set_indent_str("\t");

			// Create the config.
			let config = Config::default();

			// Create the emitter.
			let mut emitter = Emitter {
				cfg: config,
				comments: None,
				cm: source_map.clone(),
				wr: writer,
			};

			// Emit the module.
			emitter
				.emit_module(&module)
				.map_err(crate::error::Error::other)?;
			let transpiled_text =
				String::from_utf8(transpiled_text).map_err(crate::error::Error::other)?;

			// Create the source map.
			let mut output_source_map = Vec::new();
			source_map
				.build_source_map(&source_mappings)
				.to_writer(&mut output_source_map)
				.map_err(crate::error::Error::other)
				.wrap_err("Failed to create the source map.")?;
			let source_map =
				String::from_utf8(output_source_map).map_err(crate::error::Error::other)?;

			// Create the output.
			let output = Output {
				transpiled_text,
				source_map,
			};

			Ok(output)
		})
	}
}

struct TargetVisitor {
	source_map: Rc<SourceMap>,
	errors: Vec<Error>,
}

impl VisitMut for TargetVisitor {
	fn visit_mut_expr(&mut self, n: &mut Expr) {
		// Check that this is a call expression.
		let Some(expr) = n.as_mut_call() else {
			n.visit_mut_children_with(self);
			return;
		};

		// Visit the call.
		self.visit_call(expr, None);

		n.visit_mut_children_with(self);
	}

	fn visit_mut_export_default_expr(&mut self, n: &mut ExportDefaultExpr) {
		// Check that this is an await expression.
		let Some(expr) = n.expr.as_mut_await_expr() else {
			n.visit_mut_children_with(self);
			return;
		};

		// Check that this is a call expression.
		let Some(expr) = expr.arg.as_mut_call() else {
			n.visit_mut_children_with(self);
			return;
		};

		// Visit the call.
		self.visit_call(expr, Some("default".to_owned()));

		n.visit_mut_children_with(self);
	}

	fn visit_mut_export_decl(&mut self, n: &mut ExportDecl) {
		// Check that this export statement has a declaration.
		let Some(decl) = n.decl.as_mut_var() else {
			n.visit_mut_children_with(self);
			return;
		};

		// Visit each declaration.
		for decl in &mut decl.decls {
			let VarDeclarator { name, init, .. } = decl;
			let Some(ident) = name.as_ident().map(|ident| &ident.sym) else {
				continue;
			};
			let Some(init) = init.as_deref_mut() else {
				continue;
			};
			let Some(expr) = init.as_mut_await_expr() else {
				continue;
			};
			let Some(expr) = expr.arg.as_mut_call() else {
				continue;
			};

			// Visit the call.
			self.visit_call(expr, Some(ident.to_string()));
		}

		n.visit_mut_children_with(self);
	}
}

impl TargetVisitor {
	#[allow(clippy::too_many_lines)]
	fn visit_call(&mut self, n: &mut CallExpr, export_name: Option<String>) {
		// Check if this is a call to tg.target.
		let Some(callee) = n.callee.as_expr().and_then(|expr| expr.as_member()) else {
			n.visit_mut_children_with(self);
			return;
		};
		let Some(obj) = callee.obj.as_ident() else {
			n.visit_mut_children_with(self);
			return;
		};
		let Some(prop) = callee.prop.as_ident() else {
			n.visit_mut_children_with(self);
			return;
		};
		if !(&obj.sym == "tg" && &prop.sym == "target") {
			n.visit_mut_children_with(self);
			return;
		}

		// Get the location of the call.
		let loc = self.source_map.lookup_char_pos(n.span.lo);

		// Get the name and function from the call.
		let (name, f) = match n.args.len() {
			// Handle one argument.
			1 => {
				let Some(name) = export_name else {
					self.errors.push(Error::new(
						"Targets that are not exported must have a name.",
						&loc,
					));
					n.visit_mut_children_with(self);
					return;
				};
				let Some(f) = n.args[0].expr.as_arrow() else {
					self.errors.push(Error::new(
						"The argument to tg.target must be an arrow function.",
						&loc,
					));
					n.visit_mut_children_with(self);
					return;
				};
				(name, f)
			},

			// Handle two arguments.
			2 => {
				let Some(Lit::Str(name)) = n.args[0].expr.as_lit() else {
					self.errors.push(Error::new(
						"The first argument to tg.target must be a string.",
						&loc,
					));
					n.visit_mut_children_with(self);
					return;
				};
				let name = name.value.to_string();
				let Some(f) = n.args[1].expr.as_arrow() else {
					self.errors.push(Error::new(
						"The second argument to tg.target must be an arrow function.",
						&loc,
					));
					n.visit_mut_children_with(self);
					return;
				};
				(name, f)
			},

			// Any other number of arguments is invalid.
			_ => {
				self.errors.push(Error::new(
					"Invalid number of arguments to tg.target.",
					&loc,
				));
				n.visit_mut_children_with(self);
				return;
			},
		};

		// Create the function property.
		let f_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
			key: Ident::new("function".into(), n.span).into(),
			value: Box::new(f.clone().into()),
		})));

		// Create the module property.
		let import_meta = Expr::MetaProp(MetaPropExpr {
			span: DUMMY_SP,
			kind: swc_core::ecma::ast::MetaPropKind::ImportMeta,
		});
		let import_meta_module = MemberExpr {
			span: DUMMY_SP,
			obj: Box::new(import_meta),
			prop: Ident::new("module".into(), n.span).into(),
		};
		let module_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
			key: Ident::new("module".into(), n.span).into(),
			value: Box::new(import_meta_module.into()),
		})));

		// Create the name property.
		let key = Ident::new("name".into(), n.span);
		let value: Expr = Lit::Str(Str {
			value: name.into(),
			span: n.span,
			raw: None,
		})
		.into();
		let name_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
			key: key.into(),
			value: Box::new(value),
		})));

		// Create the object.
		let object = ObjectLit {
			props: vec![f_prop, module_prop, name_prop],
			span: DUMMY_SP,
		};

		// Set the args.
		n.args = vec![ExprOrSpread {
			spread: None,
			expr: object.into(),
		}];
	}
}

struct IncludeVisitor {
	source_map: Rc<SourceMap>,
	errors: Vec<Error>,
}

impl VisitMut for IncludeVisitor {
	fn visit_mut_call_expr(&mut self, n: &mut CallExpr) {
		// Ignore call expression that are not tg.include.
		let Some(callee) = n.callee.as_expr().and_then(|callee| callee.as_member()) else {
			n.visit_mut_children_with(self);
			return;
		};
		let Some(obj) = callee.obj.as_ident() else {
			n.visit_mut_children_with(self);
			return;
		};
		let Some(prop) = callee.prop.as_ident() else {
			n.visit_mut_children_with(self);
			return;
		};
		if !(&obj.sym == "tg" && &prop.sym == "include") {
			n.visit_mut_children_with(self);
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
		let Some(arg) = n.args[0].expr.as_lit() else {
			self.errors.push(Error::new(
				"The argument to tg.include must be a string literal.",
				&loc,
			));
			return;
		};

		// Create the arg.
		let import_meta = Expr::MetaProp(MetaPropExpr {
			span: DUMMY_SP,
			kind: swc_core::ecma::ast::MetaPropKind::ImportMeta,
		});
		let import_meta_module = MemberExpr {
			span: DUMMY_SP,
			obj: Box::new(import_meta),
			prop: Ident::new("module".into(), n.span).into(),
		};
		let module_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
			key: Ident::new("module".into(), n.span).into(),
			value: Box::new(import_meta_module.into()),
		})));
		let path_prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
			key: Ident::new("path".into(), n.span).into(),
			value: Box::new(arg.clone().into()),
		})));
		let object = ObjectLit {
			props: vec![module_prop, path_prop],
			span: DUMMY_SP,
		};

		// Set the args.
		n.args = vec![ExprOrSpread {
			spread: None,
			expr: object.into(),
		}];
	}
}

#[cfg(test)]
mod tests {
	use crate::module::Module;
	use indoc::indoc;

	#[test]
	fn test_export_default_target() {
		let text = indoc!(
			r#"
				export default await tg.target(() => {});
			"#
		);
		let left = Module::transpile(text.to_owned()).unwrap().transpiled_text;
		let right = indoc!(
			r#"
				export default await tg.target({
					function: ()=>{},
					module: import.meta.module,
					name: "default"
				});
			"#
		);
		assert_eq!(left, right);
	}

	#[test]
	fn test_export_named_target() {
		let text = indoc!(
			r#"
				export let named = await tg.target(() => {});
			"#
		);
		let left = Module::transpile(text.to_owned()).unwrap().transpiled_text;
		let right = indoc!(
			r#"
				export let named = await tg.target({
					function: ()=>{},
					module: import.meta.module,
					name: "named"
				});
			"#
		);
		assert_eq!(left, right);
	}

	#[test]
	fn test_named_target() {
		let text = indoc!(
			r#"
				tg.target("named", () => {});
			"#
		);
		let left = Module::transpile(text.to_owned()).unwrap().transpiled_text;
		let right = indoc!(
			r#"
				tg.target({
					function: ()=>{},
					module: import.meta.module,
					name: "named"
				});
			"#
		);
		assert_eq!(left, right);
	}

	#[test]
	fn test_include() {
		let text = indoc!(
			r#"
				tg.include("./hello_world.txt");
			"#
		);
		let left = Module::transpile(text.to_owned()).unwrap().transpiled_text;
		let right = indoc!(
			r#"
				tg.include({
					module: import.meta.module,
					path: "./hello_world.txt"
				});
			"#
		);
		assert_eq!(left, right);
	}
}
