import { assert, visit } from "./util";
import { TSESTree, parse } from "@typescript-eslint/typescript-estree";

export type Request = {
	text: string;
};

export type Response = {
	imports: Array<string>;
};

export let handle = (request: Request): Response => {
	// Parse the text.
	let ast = parse(request.text);

	// Collect all imports.
	let imports: Array<string> = [];
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
		}
	});

	return { imports };
};
