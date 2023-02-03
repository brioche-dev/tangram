use std::collections::{btree_map::Entry, BTreeMap, HashSet};

use crate::{package_specifier::PackageSpecifier, Cli};
use anyhow::{bail, Context, Result};
use rome_js_syntax::{AnyJsExpression, AnyJsImportClause, AnyJsLiteralExpression};
use rome_rowan::{AstNode, AstNodeList, AstSeparatedList};

use super::{module_specifier::ModuleSpecifier, ModuleIdentifier};

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Metadata {
	pub name: Option<String>,
	pub version: Option<String>,
	pub dependencies: BTreeMap<String, PackageSpecifier>,
	#[serde(flatten)]
	pub rest: BTreeMap<String, serde_json::Value>,
}

impl Cli {
	pub async fn get_metadata(
		&self,
		root_module_identifier: &ModuleIdentifier,
	) -> Result<Metadata> {
		// Load the root module.
		let code = self.load(root_module_identifier).await?;

		// Get the exported metadata literal value from the module.
		let mut metadata = get_export_metadata(&code)?.unwrap_or_default();

		// Keep track of modules we've already visited. This prevents an infinite loop when dealing with circular dependencies.
		let mut visited_modules = HashSet::new();

		// Keep track of modules we need to visit. We start with the root module.
		let mut unvisited_modules = vec![root_module_identifier.clone()];

		// Keep a list of external packages imported by each encountered module. The metadata will be updated to include each imported package.
		let mut external_packages = vec![];

		while let Some(module_identifier) = unvisited_modules.pop() {
			// Skip this module if we've already visited it.
			let is_unvisited = visited_modules.insert(module_identifier.clone());
			if !is_unvisited {
				continue;
			}

			// Load the module.
			let code = self.load(&module_identifier).await?;

			// Get all the import specifiers from the module.
			let import_specifiers = get_import_specifiers(&code)?;

			for import_specifier in import_specifiers {
				// Parse the import specifier as a module specifier.
				let import_specifier: ModuleSpecifier = import_specifier.parse()?;

				match import_specifier {
					ModuleSpecifier::Path { module_path } => {
						// If the import refers to another module within this package, resolve it and queue it up so we visit it in another iteration.
						let module_identifier =
							Self::resolve_path(&module_path, &module_identifier)?;
						unvisited_modules.push(module_identifier);
					},
					ModuleSpecifier::Package(specifier) => {
						// If the import refers to an external package, add it to the list.
						external_packages.push(specifier);
					},
				}
			}
		}

		// Update the metadata to add any external packages that aren't already in the `dependencies` map.
		for package_specifier in external_packages {
			// Create a key for this package specifier.
			let package_key = package_specifier.key();
			let dependency_entry = metadata.dependencies.entry(package_key.to_string());

			match dependency_entry {
				Entry::Vacant(entry) => {
					// If the key doesn't already exist, then this is a new dependency, so insert it.
					entry.insert(package_specifier);
				},
				Entry::Occupied(mut entry) => {
					// If the key does already exist, try to merge the existing dependency with the new package specifier. This returns a new package specifier that covers both the old and new specifier.
					let existing_specifier_key = entry.key();
					let existing_specifier = entry.get();
					let merged_specifier = merge_package_specifiers(
						&package_specifier,
						existing_specifier_key,
						existing_specifier,
					);

					match merged_specifier {
						PackageSpecifierMergeOutcome::Merged(merged_specifier) => {
							// If we merged the specifiers, replace the existing specifier with the merged one.
							entry.insert(merged_specifier);
						},
						PackageSpecifierMergeOutcome::NotMerged => {
							// For now, bail if the specifiers couldn't be merged.
							unimplemented!("Encountered package specifier {package_specifier:?} which has key {existing_specifier_key:?}, but key already resolves to dependency {existing_specifier:?} that cannot be merged.");
						},
					}
				},
			}
		}

		Ok(metadata)
	}
}

enum PackageSpecifierMergeOutcome {
	Merged(PackageSpecifier),
	NotMerged,
}

/// Try to merge a package specifier with an existing package specifier dependency. When a package specifies the same dependency both using an import dependency and a metadata dependency, or with two imports of the same package name, this function will try to resolve both package specifiers to the same version. If the two specifiers cannot be merged (e.g. becuase they refer to two incompatible versions of the same package), then this function will return `PackageSpecifierMergeOutcome::NotMerged`.
fn merge_package_specifiers(
	new_specifier: &PackageSpecifier,
	existing_specifier_key: &str,
	existing_specifier: &PackageSpecifier,
) -> PackageSpecifierMergeOutcome {
	// If the two specifiers are exactly the same, then the merger is just the sepcifier we already have.
	if new_specifier == existing_specifier {
		return PackageSpecifierMergeOutcome::Merged(existing_specifier.clone());
	}

	// If the new specifier doesn't have a version and matches the existing specifier key, then the new specifier uses the existing specifier as an alias.
	if let PackageSpecifier::Registry {
		name,
		version: None,
	} = new_specifier
	{
		if name == existing_specifier_key {
			return PackageSpecifierMergeOutcome::Merged(existing_specifier.clone());
		}
	}

	// If the new specifier and existing specifier have the same name, but only one specifier has a version, then we merge by using the version from the specifier that has it.
	match (new_specifier, existing_specifier) {
		(
			PackageSpecifier::Registry {
				name: new_name,
				version: None,
			},
			PackageSpecifier::Registry {
				name: existing_name,
				version: Some(version),
			},
		)
		| (
			PackageSpecifier::Registry {
				name: new_name,
				version: Some(version),
			},
			PackageSpecifier::Registry {
				name: existing_name,
				version: None,
			},
		) => {
			if new_name == existing_name {
				return PackageSpecifierMergeOutcome::Merged(PackageSpecifier::Registry {
					name: new_name.clone(),
					version: Some(version.clone()),
				});
			}
		},
		_ => {},
	}

	// In any other case, we don't merge the package specifiers.
	PackageSpecifierMergeOutcome::NotMerged
}

/// Get the metadata exported from the given source code of a TypeScript module. If the module doesn't export any metadata, then this function returns `Ok(None)`.
fn get_export_metadata(code: &str) -> Result<Option<Metadata>> {
	// Parse the source code as a TypeScript module.
	let source_type = rome_js_syntax::SourceType::ts();
	let source_ast_root = rome_js_parser::parse(code, source_type).tree();
	let module = source_ast_root
		.as_js_module()
		.context("Expected code to be a module.")?;

	// Find an export declaraction named `metadata`.
	let export_metadata_initializer = module.items().iter().find_map(|item| {
		// Filter to export declarations.
		let export = item.as_js_export()?;
		let export_clause = export.export_clause().ok()?;
		let export_declaration = export_clause
			.as_any_js_declaration_clause()?
			.as_js_variable_declaration_clause()?;

		// Find the export declarator named `metadata` with an initializer.
		let export_metadata_initializer = export_declaration
			.declaration()
			.ok()?
			.declarators()
			.iter()
			.find_map(|declarator| {
				// Get the name of the export declarator.
				let declarator = declarator.ok()?;
				let binding = declarator.id().ok()?;
				let binding = binding.as_any_js_binding()?;
				let binding = binding.as_js_identifier_binding()?;
				let binding_name = binding.name_token().ok()?;
				let binding_name = binding_name.text_trimmed();

				// Return the initializer if the name is `metadata`.
				if binding_name == "metadata" {
					let initializer = declarator.initializer()?;
					Some(initializer)
				} else {
					None
				}
			})?;

		Some(export_metadata_initializer)
	});

	// If no `export` declaration was found, return `None`.
	let Some(export_metadata_initializer) = export_metadata_initializer else {
		return Ok(None);
	};

	// Get the exported metadata JS expression.
	let export_metadata = export_metadata_initializer
		.expression()
		.context("Invalid expression for exported metadata.")?;

	// Convert the metadata expression into JSON.
	let export_metadata_json = js_expression_to_json(&export_metadata)?;

	// Parse the JSON into a `Metadata` struct.
	let export_metadata = serde_json::from_value(export_metadata_json)
		.context("Invalid value for exported metadata.")?;

	Ok(export_metadata)
}

fn get_import_specifiers(code: &str) -> Result<Vec<String>> {
	let source_type = rome_js_syntax::SourceType::ts();
	let source_ast_root = rome_js_parser::parse(code, source_type).tree();

	let module = source_ast_root
		.as_js_module()
		.context("Expected code to be a module.")?;

	let import_specifiers = module.items().iter().filter_map(|item| {
		let import = item.as_js_import()?;
		let import_clause = import.import_clause().ok()?;
		let import_source = match import_clause {
			AnyJsImportClause::JsImportBareClause(import) => import.source(),
			AnyJsImportClause::JsImportDefaultClause(import) => import.source(),
			AnyJsImportClause::JsImportNamedClause(import) => import.source(),
			AnyJsImportClause::JsImportNamespaceClause(import) => import.source(),
		};
		let import_source = import_source.ok()?;
		let import_text = import_source.inner_string_text().ok()?;

		Some(import_text.to_string())
	});

	Ok(import_specifiers.collect())
}

/// Convert a JavaScript expression into a JSON value statically. This only supports a small subset of JavaScript expressions that can be trivially converted to JSON, such as object literals, array literals, and string literals.
fn js_expression_to_json(expression: &AnyJsExpression) -> Result<serde_json::Value> {
	let value = match expression {
		AnyJsExpression::AnyJsLiteralExpression(
			AnyJsLiteralExpression::JsBigIntLiteralExpression { .. },
		) => {
			// BigInt literals have no JSON equivalent.
			bail!("BigInt literals are not supported.");
		},
		AnyJsExpression::AnyJsLiteralExpression(
			literal @ AnyJsLiteralExpression::JsBooleanLiteralExpression { .. },
		) => {
			// Convert a boolean literal by name.
			match &*literal.text() {
				"true" => serde_json::Value::Bool(true),
				"false" => serde_json::Value::Bool(false),
				_ => bail!("Invalid boolean literal."),
			}
		},
		AnyJsExpression::AnyJsLiteralExpression(
			AnyJsLiteralExpression::JsNullLiteralExpression { .. },
		) => {
			// Map `null` to a JSON null.
			serde_json::Value::Null
		},
		AnyJsExpression::AnyJsLiteralExpression(
			AnyJsLiteralExpression::JsNumberLiteralExpression { .. },
		) => {
			// Parse the number literal as a JSON number.
			let text = expression.text();
			let value = text
				.parse::<serde_json::Number>()
				.context("Invalid number literal.")?;
			serde_json::Value::Number(value)
		},
		AnyJsExpression::AnyJsLiteralExpression(
			AnyJsLiteralExpression::JsRegexLiteralExpression { .. },
		) => {
			// Regular expression literals have no JSON equivalent.
			bail!("Regular expression literals are not supported.");
		},
		AnyJsExpression::AnyJsLiteralExpression(
			AnyJsLiteralExpression::JsStringLiteralExpression(literal),
		) => {
			// Convert the string literal to a JSON string.
			let string_text = literal.inner_string_text()?;
			serde_json::Value::String(string_text.to_string())
		},
		AnyJsExpression::JsObjectExpression(object_expression) => {
			let mut object = serde_json::Map::new();

			for member in object_expression.members().iter() {
				// Get the property member. This filters out method members, getters and setters, the spread operator, etc.
				let member = member?;
				let member = member
					.as_js_property_object_member()
					.context("Only propery fields are supported.")?;

				// Get the literal property name. This filters out computed member names.
				let name = member.name()?;
				let name = name
					.as_js_literal_member_name()
					.context("Only literal object property names are supported.")?;
				let name = name.name()?;

				// Convert the value to a JSON value.
				let value = member.value()?;
				let value = js_expression_to_json(&value)?;

				object.insert(name, value);
			}

			serde_json::Value::Object(object)
		},
		AnyJsExpression::JsArrayExpression(array_expression) => {
			let mut array = Vec::new();

			for element in array_expression.elements().iter() {
				// Get the element. This filters out the spread operator and holes in the array.
				let element = element?;
				let element = element
					.as_any_js_expression()
					.context("Only literal array elements are supported.")?;
				let element = js_expression_to_json(element)?;

				array.push(element);
			}

			serde_json::Value::Array(array)
		},
		AnyJsExpression::JsParenthesizedExpression(parenthesized_expression) => {
			// Convert the expression inside the parentheses.
			let expression = parenthesized_expression.expression()?;
			js_expression_to_json(&expression)?
		},
		expression => {
			bail!(
				"Could not convert JavaScript expression to JSON: {:?}.",
				expression.text()
			);
		},
	};

	Ok(value)
}
