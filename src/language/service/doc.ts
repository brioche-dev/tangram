/* eslint-disable @typescript-eslint/no-non-null-assertion */

import { Location } from "./location.ts";
import { ModuleIdentifier } from "./syscall.ts";
import { compilerOptions, host } from "./typescript.ts";
import * as typescript from "./typescript.ts";
import ts from "typescript";

export type Request = {
	moduleIdentifier: ModuleIdentifier;
};

export type Response = {
	doc: Doc;
};

type Doc = {
	exports: Array<Export>;
};

type Export = TypeExport | VariableExport;

type TypeExport = {
	kind: "type";
	location: Location;
	name: string;
	typeParameters: Array<TypeParameter>;
	type: Type;
};

type VariableExport = {
	kind: "variable";
	location: Location;
	name: string;
	type: Type;
};

type TypeParameter = {
	name: string;
};

type Type =
	| LiteralType
	| KeywordType
	| ReferenceType
	| UnionType
	| IntersectionType
	| TupleType
	| ObjectType
	| FunctionType;

type LiteralType = {
	kind: "literal";
	value: string;
};

type KeywordType = {
	kind: "keyword";
	name:
		| "any"
		| "bigint"
		| "boolean"
		| "never"
		| "null"
		| "number"
		| "string"
		| "symbol"
		| "undefined"
		| "unknown"
		| "void";
};

type ReferenceType = {
	kind: "reference";
	location: Location;
	name: string;
	typeArguments: Array<Type> | undefined;
};

type UnionType = {
	kind: "union";
	types: Array<Type>;
};

type IntersectionType = {
	kind: "union";
	types: Array<Type>;
};

type TupleType = {
	kind: "array";
	types: Array<Type>;
};

type ObjectType = {
	kind: "object";
	properties: Array<PropertyType>;
};

type PropertyType = {
	name: string;
	type: Type;
};

type FunctionType = {
	kind: "function";
	location: Location;
	parameters: Array<Parameter>;
	typeParameters: Array<TypeParameter>;
	return: Type;
};

type Parameter = {
	name: string;
	optional: boolean;
	type: Type;
};

export let handle = (request: Request): Response => {
	// Create the program and type checker.
	let program = ts.createProgram({
		rootNames: [
			typescript.fileNameFromModuleIdentifier(request.moduleIdentifier),
		],
		options: compilerOptions,
		host,
	});
	let typeChecker = program.getTypeChecker();

	// Get the module's exports.
	let sourceFile = program.getSourceFile(
		typescript.fileNameFromModuleIdentifier(request.moduleIdentifier),
	)!;
	let symbol = typeChecker.getSymbolAtLocation(sourceFile)!;
	let moduleExports = typeChecker.getExportsOfModule(symbol);

	// Convert the exports.
	let exports = [];
	for (let moduleExport of moduleExports) {
		exports.push(convertExport(typeChecker, moduleExport));
	}

	// Create the doc.
	let doc = {
		exports,
	};

	return {
		doc,
	};
};

let convertExport = (
	typeChecker: ts.TypeChecker,
	moduleExport: ts.Symbol,
): Export => {
	throw new Error();
};
