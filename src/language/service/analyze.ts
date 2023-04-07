import { assert, visit } from "./util.ts";
import { TSESTree, parse } from "@typescript-eslint/typescript-estree";

export type Request = {
	text: string;
};

export type Response = {
	imports: Array<string>;
	includes: Array<string>;
};

export let handle = (request: Request): Response => {
	// Parse the text.
	let ast = parse(request.text);

	// Collect all imports and includes.
	let imports: Array<string> = [];
	let includes: Array<string> = [];
	visit(ast, (node: TSESTree.Node) => {
		switch (node.type) {
			case TSESTree.AST_NODE_TYPES.ImportDeclaration: {
				assert(typeof node.source.value === "string");
				imports.push(node.source.value);
				break;
			}

			case TSESTree.AST_NODE_TYPES.ImportExpression: {
				assert(node.source.type === TSESTree.AST_NODE_TYPES.Literal);
				assert(typeof node.source.value === "string");
				imports.push(node.source.value);
				break;
			}

			case TSESTree.AST_NODE_TYPES.ExportNamedDeclaration: {
				if (node.source) {
					imports.push(node.source.value);
				}
				break;
			}

			case TSESTree.AST_NODE_TYPES.ExportAllDeclaration: {
				imports.push(node.source.value);
				break;
			}

			case TSESTree.AST_NODE_TYPES.CallExpression: {
				// Ensure the callee is a member expression.
				if (node.callee.type !== TSESTree.AST_NODE_TYPES.MemberExpression) {
					break;
				}

				// Ensure the callee is `tg.include`.
				if (
					node.callee.object.type !== TSESTree.AST_NODE_TYPES.Identifier ||
					node.callee.object.name !== "tg" ||
					node.callee.property.type !== TSESTree.AST_NODE_TYPES.Identifier ||
					node.callee.property.name !== "include"
				) {
					break;
				}

				// Require that there is one argument that is a string.
				assert(node.arguments.length === 1);
				let argument = node.arguments.at(0);
				assert(argument);
				assert(argument.type === TSESTree.AST_NODE_TYPES.Literal);
				assert(typeof argument.value === "string");
				includes.push(argument.value);

				break;
			}
		}
	});

	return { imports, includes };
};
