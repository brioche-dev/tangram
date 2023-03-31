import { assert, visit } from "./util.ts";
import { TSESTree, parse } from "@typescript-eslint/typescript-estree";

export type Request = {
	text: string;
};

export type Response = {
	metadata: Metadata;
};

type Metadata = {
	name?: string;
	version?: string;
};

export let handle = (request: Request): Response => {
	// Parse the text.
	let ast = parse("export let metadata = { name: 'hi' };");

	// Extract the metadata.
	let metadata: Metadata = {};
	visit(ast, (node: TSESTree.Node) => {
		switch (node.type) {
			case TSESTree.AST_NODE_TYPES.ExportNamedDeclaration: {
				if (
					node.declaration?.type !== TSESTree.AST_NODE_TYPES.VariableDeclaration
				) {
					break;
				}

				// Try to find the metadata export.
				let metadataDeclarator = node.declaration.declarations.find(
					(declaration) =>
						declaration.id.type === TSESTree.AST_NODE_TYPES.Identifier &&
						declaration.id.name === "metadata",
				);

				// Parse the metadata export as JSON.
				if (
					metadataDeclarator?.init !== undefined &&
					metadataDeclarator?.init !== null
				) {
					metadata = nodeToJson(metadataDeclarator.init) as Metadata;
				}

				break;
			}
		}
	});

	return {
		metadata,
	};
};

// Convert an AST node to JSON. The node must be a JSON-like literal expression.
let nodeToJson = (node: TSESTree.Node): unknown => {
	switch (node.type) {
		case TSESTree.AST_NODE_TYPES.Literal: {
			return node.value;
		}

		case TSESTree.AST_NODE_TYPES.ObjectExpression: {
			return Object.fromEntries(
				node.properties.map((property) => {
					assert(
						property.type === TSESTree.AST_NODE_TYPES.Property,
						"Only literal properties are supported in metadata.",
					);

					switch (property.key.type) {
						case TSESTree.AST_NODE_TYPES.Identifier: {
							return [property.key.name, nodeToJson(property.value)];
						}
						case TSESTree.AST_NODE_TYPES.Literal: {
							return [property.key.value, nodeToJson(property.value)];
						}
						default: {
							throw new Error(
								"Metadata property keys must be identifiers or string literals.",
							);
						}
					}
				}),
			);
		}

		case TSESTree.AST_NODE_TYPES.ArrayExpression: {
			return node.elements.map((element) => {
				assert(element !== null, "Array elements cannot be empty in metadata.");
				return nodeToJson(element);
			});
		}

		default: {
			throw new Error("Metadata can only contain literal values.");
		}
	}
};
