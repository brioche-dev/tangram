import { Location } from "./location.ts";
import { Range } from "./range.ts";
import { Module } from "./syscall.ts";
import * as typescript from "./typescript.ts";
import ts from "typescript";

export type Request = {
	module: Module;
};

export type Response = {
	symbols: Array<Symbol> | null;
};

export type Symbol = {
	name: string;
	detail: string | null;
	kind: Kind;
	tags: Array<Tag>;
	range: Range;
	selectionRange: Range;
	children: Array<Symbol> | null;
};

export type Kind =
	| "unknown"
	| "file"
	| "module"
	| "namespace"
	| "package"
	| "class"
	| "method"
	| "property"
	| "field"
	| "constructor"
	| "enum"
	| "interface"
	| "function"
	| "variable"
	| "constant"
	| "string"
	| "number"
	| "boolean"
	| "array"
	| "object"
	| "key"
	| "null"
	| "enumMember"
	| "event"
	| "operator"
	| "typeParameter";

export type Tag = "deprecated";

export let handle = (request: Request): Response => {
	// Get the source file and position.
	let sourceFile = typescript.host.getSourceFile(
		typescript.fileNameFromModule(request.module),
		ts.ScriptTarget.ESNext,
	);

	if (sourceFile === undefined) {
		throw new Error();
	}

	let symbols = [];

	// Get the navigation tree for this file.
	let navigationTree = typescript.languageService.getNavigationTree(sourceFile);

	// Get the symbols by walking the navigation tree.
	let symbols = [walk(navigationTree)];

	return { symbols }
};

// https://github.com/microsoft/TypeScript/blob/59d3a381807bb4247a36a24be7e41553ebe6d8b5/src/services/types.ts#L826
export let walk = (file: string, tree: typescript.NavigationTree): Symbol => {
	let name = tree.text;

	// Find the range of this symbol and its selectionRange.
	let span = tree
		.span
		.reduce((acc, span) => {
			acc.start = Math.min(acc.start, span.start);
			acc.end = Math.max(acc.end, span.end);
		});
	let range = spanToRange(file, span);
	let selectionRange = spanToRange(file, tree.nameSpan ?? span);

	// Parse the symbol kind from the nav tree.
	let kind = getKind(tree.kind);

	// Collect the nested children.
	let children = tree.children?.map(walk);

	return {
		name,
		kind,
		tags: [], // TODO: deprecation tags.
		detail: null, // TODO: symbol details.
		range,
		selectionRange,
		children,
	};
};

// https://github.com/microsoft/TypeScript/blob/59d3a381807bb4247a36a24be7e41553ebe6d8b5/src/services/types.ts#L1559
let getKind = (tsKind:string): Kind => {
	switch(tsKind) {
		case "script": {
			kind = "file";
			break;
		}
		case "module": {
			kind = "module";
			break;
		}
		case "class": {
			kind = "class";
			break;
		}
		case "local class": {
			kind = "class";
			break;
		}
		case "interface": {
			kind = "interface";
			break;
		}
		case "type": {
			kind = "class";
			break;
		}
		case "enum": {
			kind = "class";
			break;
		}
		case "var": {
			kind = "variable";
			break;
		}
		case "local var": {
			kind = "variable";
			break;
		}
		case "function": {
			kind = "function";
			break;
		}
		case "local function": {
			kind = "function";
			break;
		}
		case "method": {
			kind = "method";
			break;
		}
		case "getter": {
			kind = "method";
			break;
		}
		case "setter": {
			kind = "method";
			break;
		}
		case "property": {
			kind = "property";
			break;
		}
		case "accessor": {
			kind = "property";
			break;
		}
		case "constructor": {
			kind = "constructor";
			break;
		}
		case "parameter": {
			kind = "variable";
			break;
		}
		case "type parameter": {
			kind = "typeParameter";
			break;
		}
		case "external module": {
			kind = "module";
			break;
		}
		default: {
			kind = "unknown";
			break;
		}
	}
}
