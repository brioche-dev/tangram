/* eslint-disable @typescript-eslint/no-non-null-assertion */

import { Range } from "./range.ts";
import { Module } from "./syscall.ts";
import { compilerOptions, host } from "./typescript.ts";
import * as typescript from "./typescript.ts";
import ts from "typescript";

export type Request = {
	module: Module;
};

export type Response = {
	doc: Doc;
};

type Doc = {
	exports: Array<Export>;
};

type Location = {
	module: DocModule;
	range: Range;
};

type DocModule =
	| { kind: "library"; value: Library }
	| { kind: "normal"; value: NormalDocModule };

type Library = {
	modulePath: string;
};

type NormalDocModule = {
	packageHash: string;
	modulePath: string;
};

type Export =
	| TypeExport
	| VariableExport
	| FunctionExport
	| ClassExport
	| NamespaceExport
	| EnumExport
	| InterfaceExport;

type TypeExport = {
	kind: "type";
	location: Location;
	name: string;
	typeParameters: Array<TypeParameter>;
	type: Type;
	comment: Comment;
};

type VariableExport = {
	kind: "variable";
	location: Location;
	name: string;
	type: Type;
	comment: Comment;
};

type FunctionExport = {
	kind: "function";
	location: Location;
	name: string;
	type: Type;
};

type ClassExport = {
	kind: "class";
	location: Location;
	name: string;
	properties: Array<PropertyType>;
	constructor_?: Constructor;
};

type Constructor = {
	kind: "constructor";
	parameters: Array<Parameter>;
	return: Type;
};

type NamespaceExport = {
	kind: "namespace";
	location: Location;
	name: string;
	exports: Array<Export>;
};

type EnumExport = {
	kind: "enum";
	location: Location;
	name: string;
	members: Array<EnumMember>;
};

type EnumMember = {
	name: string;
	value: StringLiteralType | NumberLiteralType | UnknownLiteralType;
};

type InterfaceExport = {
	kind: "interface";
	name: string;
	location: Location;
	// TODO
	members: Array<any>;
};

type Type =
	| LiteralType
	| KeywordType
	| ReferenceType
	| UnionType
	| IntersectionType
	| TupleType
	| ArrayType
	| Object
	| FunctionType
	| InferType
	| ConditionalType
	| MappedType
	| TemplateLiteralType
	| IndexedAccessType
	| TypeQueryType
	| TypeOperatorType
	| PredicateType
	| { kind: "_unknown"; value: string };

type LiteralType = {
	kind: "literal";
	type:
		| StringLiteralType
		| NumberLiteralType
		| NullLiteralType
		| BooleanLiteralType
		| UnknownLiteralType;
};

type StringLiteralType = {
	kind: "string";
	value: string;
};

type NumberLiteralType = {
	kind: "number";
	value: number;
};

type NullLiteralType = {
	kind: "null";
};

type BooleanLiteralType = {
	kind: "boolean";
	value: boolean;
};

type UnknownLiteralType = {
	kind: "unknown";
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
	exported: boolean;
	name: string;
	typeArguments: Array<Type>;
};

type UnionType = {
	kind: "union";
	types: Array<Type>;
};

type TupleType = {
	kind: "tuple";
	types: Array<Type>;
};

type IntersectionType = {
	kind: "intersection";
	types: Array<Type>;
};

type ArrayType = {
	kind: "array";
	type: Type;
};

type Object = {
	kind: "object";
	properties: Array<PropertyType>;
	indexSignature?: IndexSignature;
};

type IndexSignature = {
	key: Parameter;
	type: Type;
};

type PropertyType = {
	name: string;
	type: Type;
	static?: boolean;
	private?: boolean;
};

type FunctionType = {
	kind: "function";
	parameters: Array<Parameter>;
	typeParameters: Array<TypeParameter>;
	return: Type;
};

type ConditionalType = {
	kind: "conditional";
	checkType: Type;
	extendsType: Type;
	trueType: Type;
	falseType: Type;
};

type InferType = {
	kind: "infer";
	typeParameter: TypeParameter;
};

type MappedType = {
	kind: "mapped";
	type: Type;
	typeParameterName: string;
	constraint: Type;
	nameType?: Type;
};

type IndexedAccessType = {
	kind: "indexed_access";
	objectType: Type;
	indexType: Type;
};

type TemplateLiteralType = {
	kind: "template_literal";
	head: string;
	templateSpans: Array<TemplateLiteralTypeSpan>;
};

type TemplateLiteralTypeSpan = {
	type: Type;
	literal: string;
};

type TypeQueryType = {
	kind: "type_query";
	name: string;
	location: Location;
};

type TypeOperatorType = {
	kind: "type_operator";
	operator: string;
	type: Type;
};

type PredicateType = {
	kind: "predicate";
	name: string;
	asserts: boolean;
	type?: Type;
};

type Parameter = {
	name: string;
	optional: boolean;
	type: Type;
};

type TypeParameter = {
	name: string;
	default?: Type;
	constraint?: Type;
};

type Comment = {
	summary: string;
	tags: Array<{ name: string; comment: string }>;
};

let convertSymbol = (
	typeChecker: ts.TypeChecker,
	moduleExport: ts.Symbol,
): Array<Export> => {
	let exports: Array<Export> = [];

	// Get the flags.
	let symbolFlags = moduleExport.getFlags();

	// Namespace Module.
	if (ts.SymbolFlags.NamespaceModule & symbolFlags) {
		exports.push(convertModuleSymbol(typeChecker, moduleExport));
	}

	// Value Module.
	if (ts.SymbolFlags.ValueModule & symbolFlags) {
		exports.push(convertModuleSymbol(typeChecker, moduleExport));
	}

	// Class.
	if (ts.SymbolFlags.Class & symbolFlags) {
		exports.push(convertClassSymbol(typeChecker, moduleExport));
	}

	// Variable.
	if (ts.SymbolFlags.Variable & symbolFlags) {
		exports.push(convertVariableSymbol(typeChecker, moduleExport));
	}

	// Function.
	if (ts.SymbolFlags.Function & symbolFlags) {
		exports.push(convertFunctionSymbol(typeChecker, moduleExport));
	}

	// TypeAlias.
	if (ts.SymbolFlags.TypeAlias & symbolFlags) {
		exports.push(convertTypeAliasSymbol(typeChecker, moduleExport));
	}

	// Alias.
	if (ts.SymbolFlags.Alias & symbolFlags) {
		exports.push(...convertAliasSymbol(typeChecker, moduleExport));
	}

	// Enum.
	if (ts.SymbolFlags.Enum & symbolFlags) {
		exports.push(convertEnumSymbol(typeChecker, moduleExport));
	}

	// Interface.
	if (
		ts.SymbolFlags.Interface & symbolFlags &&
		!(ts.SymbolFlags.Class & symbolFlags)
	) {
		exports.push(convertInterfaceSymbol(typeChecker, moduleExport));
	}

	// Handle default export Property as Variable.
	if (
		ts.SymbolFlags.Property & moduleExport.flags &&
		moduleExport.getName() == "default"
	) {
		exports.push(convertVariableSymbol(typeChecker, moduleExport));
	}

	return exports;
};

// A NamespaceModule is a namespace that contains ONLY types.
// A ValueModule is a module that contains values.
let convertModuleSymbol = (
	typeChecker: ts.TypeChecker,
	symbol: ts.Symbol,
): NamespaceExport => {
	// Convert the exports of the namespace.
	let exports: Array<Export> = [];
	let namespaceExports = typeChecker.getExportsOfModule(symbol);
	for (let namespaceExport of namespaceExports) {
		let flags = namespaceExport.getFlags();
		if (flags & ts.SymbolFlags.ModuleMember) {
			exports.push(...convertSymbol(typeChecker, namespaceExport));
		}
	}

	// Get a location for one of the namespace declarations.
	let declaration = symbol
		.getDeclarations()
		?.find((d): d is ts.ModuleDeclaration => ts.isModuleDeclaration(d));
	if (!declaration) {
		throw new Error("");
	}

	return {
		kind: "namespace",
		name: symbol.getName(),
		location: convertLocation(declaration),
		exports,
	};
};

let convertClassSymbol = (
	typeChecker: ts.TypeChecker,
	symbol: ts.Symbol,
): ClassExport => {
	// Get the class declaration.
	let declaration = symbol
		.getDeclarations()
		?.find((d) => ts.isClassDeclaration(d));
	if (!declaration) {
		throw new Error("");
	}

	let properties: Array<PropertyType> = [];

	// Get the instance type properties of the class.
	let instanceType = typeChecker.getDeclaredTypeOfSymbol(symbol);
	for (let instanceProperty of typeChecker.getPropertiesOfType(instanceType)) {
		properties.push(convertPropertySymbol(typeChecker, instanceProperty));
	}

	// Get the static properties of the class.
	let staticType = typeChecker.getTypeOfSymbolAtLocation(symbol, declaration);
	for (let staticProperty of typeChecker.getPropertiesOfType(staticType)) {
		// Ignore prototype.
		if (staticProperty.flags & ts.SymbolFlags.Prototype) continue;
		properties.push(convertPropertySymbol(typeChecker, staticProperty));
	}

	// Get the constructor signature if there is one.
	let constructSignature = staticType.getConstructSignatures()[0]!;
	let constructor: Constructor | undefined;
	if (constructSignature.getDeclaration()) {
		let parameters = constructSignature.getParameters().map((parameter) => {
			let parameterType = typeChecker.getTypeOfSymbol(parameter);
			let declaration: ts.ParameterDeclaration | undefined =
				parameter.valueDeclaration as ts.ParameterDeclaration;
			let optional = false;
			if (declaration) {
				if (ts.isParameter(declaration) && declaration.questionToken) {
					optional = true;
				}
			}
			let comment = ts.displayPartsToString(
				parameter.getDocumentationComment(typeChecker),
			);
			return {
				name: parameter.getName(),
				type: declaration?.type
					? convertTypeNode(typeChecker, declaration.type)
					: convertType(typeChecker, parameterType),
				optional,
				comment,
			};
		});

		constructor = {
			kind: "constructor",
			parameters,
			return: convertType(typeChecker, constructSignature.getReturnType()),
		};
	}

	return {
		name: symbol.getName(),
		kind: "class",
		location: convertLocation(declaration),
		properties,
		constructor_: constructor,
	};
};

let convertVariableSymbol = (
	typeChecker: ts.TypeChecker,
	symbol: ts.Symbol,
): VariableExport => {
	// Get the declaration.
	let declaration = symbol.getDeclarations()?.[0];
	if (!declaration) {
		throw new Error("");
	}

	// Convert the declaration.
	let type: Type;
	if (ts.isVariableDeclaration(declaration) && declaration.type) {
		type = convertTypeNode(typeChecker, declaration.type);
	} else {
		type = convertType(typeChecker, typeChecker.getTypeOfSymbol(symbol));
	}

	return {
		kind: "variable",
		name: symbol.getName(),
		location: convertLocation(declaration),
		type,
		comment: convertComment(typeChecker, symbol),
	};
};

let convertFunctionSymbol = (
	typeChecker: ts.TypeChecker,
	symbol: ts.Symbol,
): FunctionExport => {
	// Get the declaration.
	let declaration = symbol
		.getDeclarations()
		?.find((d): d is ts.FunctionDeclaration => ts.isFunctionDeclaration(d));
	if (!declaration) {
		throw new Error("");
	}

	// Convert the declaration.
	let type: Type;
	if (declaration.type) {
		type = convertTypeNode(typeChecker, declaration.type);
	} else {
		type = convertType(typeChecker, typeChecker.getTypeOfSymbol(symbol));
	}
	return {
		kind: "function",
		location: convertLocation(declaration),
		name: symbol.getName(),
		type,
	};
};

let convertTypeAliasSymbol = (
	typeChecker: ts.TypeChecker,
	symbol: ts.Symbol,
): TypeExport => {
	// Get the declaration.
	let declaration = symbol
		.getDeclarations()
		?.find((d): d is ts.TypeAliasDeclaration => ts.isTypeAliasDeclaration(d));
	if (!declaration) {
		throw new Error("");
	}

	// Convert the declaration.
	let type = convertTypeNode(typeChecker, declaration.type);
	let typeParameters = declaration.typeParameters?.map((typeParameter) =>
		convertTypeParameterNode(typeChecker, typeParameter),
	);

	return {
		kind: "type",
		name: symbol.getName(),
		location: convertLocation(declaration),
		type,
		typeParameters: typeParameters ?? [],
		comment: convertComment(typeChecker, symbol),
	};
};

let convertAliasSymbol = (
	typeChecker: ts.TypeChecker,
	symbol: ts.Symbol,
): Array<Export> => {
	let originalName = symbol.getName();
	let exports = convertSymbol(
		typeChecker,
		getAliasedSymbolIfAliased(typeChecker, symbol),
	).map((e) => {
		return { ...e, name: originalName };
	});
	return exports;
};

let convertEnumSymbol = (
	typeChecker: ts.TypeChecker,
	symbol: ts.Symbol,
): EnumExport => {
	let enumExports = typeChecker.getExportsOfModule(symbol);
	let members: Array<EnumMember> = [];
	for (let enumExport of enumExports) {
		if (enumExport.flags & ts.SymbolFlags.EnumMember) {
			members.push(convertEnumMemberSymbol(typeChecker, enumExport));
		}
	}
	let declaration = symbol.declarations![0]!;
	return {
		kind: "enum",
		name: symbol.getName(),
		location: convertLocation(declaration),
		members,
	};
};

let convertInterfaceSymbol = (
	typeChecker: ts.TypeChecker,
	symbol: ts.Symbol,
): InterfaceExport => {
	// Get the declaration.
	let declaration = symbol
		.getDeclarations()
		?.find((d) => ts.isInterfaceDeclaration(d));
	if (!declaration) {
		throw new Error("");
	}

	// Get the instance type.
	let instanceType = typeChecker.getDeclaredTypeOfSymbol(symbol);

	// Get the members.
	let members = typeChecker
		.getPropertiesOfType(instanceType)
		.map((property) => {
			return convertPropertySymbol(typeChecker, property);
		});

	return {
		kind: "interface",
		name: symbol.getName(),
		location: convertLocation(declaration),
		members,
	};
};

let convertEnumMemberSymbol = (
	typeChecker: ts.TypeChecker,
	symbol: ts.Symbol,
): EnumMember => {
	// Get the declaration.
	let declaration = symbol.getDeclarations()![0]! as ts.EnumMember;

	// Get the constant value.
	let constantValue = typeChecker.getConstantValue(declaration);

	// Convert the value.
	let value: StringLiteralType | NumberLiteralType | UnknownLiteralType;
	if (typeof constantValue == "string") {
		value = {
			kind: "string",
			value: constantValue,
		};
	} else if (typeof constantValue == "number") {
		value = {
			kind: "number",
			value: constantValue,
		};
	} else {
		value = {
			kind: "unknown",
			value: "unknown",
		};
	}

	return {
		name: symbol.getName(),
		value,
	};
};

let convertPropertySymbol = (
	typeChecker: ts.TypeChecker,
	property: ts.Symbol,
): PropertyType => {
	let flags = ts.getCombinedModifierFlags(property.valueDeclaration!);
	let type = convertType(typeChecker, typeChecker.getTypeOfSymbol(property));
	return {
		name: property.getName(),
		type,
		static: (flags & ts.ModifierFlags.Static) !== 0,
		private: (flags & ts.ModifierFlags.Private) !== 0,
	};
};

let convertTypeNode = (
	typeChecker: ts.TypeChecker,
	node: ts.TypeNode,
): Type => {
	if (keywordSet.has(node.kind)) {
		return convertKeyword(node.kind);
	} else if (ts.isTypeLiteralNode(node)) {
		return convertObjectTypeNode(typeChecker, node);
	} else if (ts.isTypeReferenceNode(node)) {
		return convertTypeReferenceTypeNode(typeChecker, node);
	} else if (ts.isArrayTypeNode(node)) {
		return convertArrayTypeNode(typeChecker, node);
	} else if (ts.isTupleTypeNode(node)) {
		return convertTupleTypeNode(typeChecker, node);
	} else if (ts.isIntersectionTypeNode(node)) {
		return convertIntersectionTypeNode(typeChecker, node);
	} else if (ts.isUnionTypeNode(node)) {
		return convertUnionTypeNode(typeChecker, node);
	} else if (ts.isFunctionTypeNode(node)) {
		return convertFunctionTypeNode(typeChecker, node);
	} else if (ts.isLiteralTypeNode(node)) {
		return convertLiteralTypeNode(typeChecker, node);
	} else if (ts.isConditionalTypeNode(node)) {
		return convertConditionalTypeNode(typeChecker, node);
	} else if (ts.isInferTypeNode(node)) {
		return convertInferTypeNode(typeChecker, node);
	} else if (ts.isMappedTypeNode(node)) {
		return convertMappedTypeNode(typeChecker, node);
	} else if (ts.isIndexedAccessTypeNode(node)) {
		return convertIndexedAccessTypeNode(typeChecker, node);
	} else if (ts.isTemplateLiteralTypeNode(node)) {
		return convertTemplateLiteralTypeNode(typeChecker, node);
	} else if (ts.isTypeQueryNode(node)) {
		return convertTypeQueryNode(typeChecker, node);
	} else if (ts.isTypeOperatorNode(node)) {
		return convertTypeOperatorNode(typeChecker, node);
	} else if (ts.isTypePredicateNode(node)) {
		return convertTypePredicateNode(typeChecker, node);
	} else {
		let type = typeChecker.getTypeFromTypeNode(node);
		return { kind: "_unknown", value: typeChecker.typeToString(type) };
	}
};

let convertType = (typeChecker: ts.TypeChecker, type: ts.Type): Type => {
	let node = typeChecker.typeToTypeNode(
		type,
		undefined,
		ts.NodeBuilderFlags.IgnoreErrors,
	)!;
	if (keywordSet.has(node.kind)) {
		return convertKeyword(node.kind);
	} else if (node.kind === ts.SyntaxKind.TypeLiteral) {
		return convertObjectType(typeChecker, type);
	} else if (node.kind === ts.SyntaxKind.TypeReference) {
		return convertTypeReferenceType(typeChecker, type);
	} else if (node.kind === ts.SyntaxKind.ArrayType) {
		return convertArrayType(typeChecker, type);
	} else if (node.kind === ts.SyntaxKind.TupleType) {
		return convertTupleType(typeChecker, type as ts.TupleType);
	} else if (node.kind === ts.SyntaxKind.IntersectionType) {
		return convertIntersectionType(typeChecker, type as ts.IntersectionType);
	} else if (node.kind === ts.SyntaxKind.UnionType) {
		return convertUnionType(typeChecker, type as ts.UnionType);
	} else if (node.kind === ts.SyntaxKind.FunctionType) {
		return convertFunctionType(typeChecker, type);
	} else if (node.kind === ts.SyntaxKind.LiteralType) {
		return convertLiteralTypeNode(typeChecker, node as ts.LiteralTypeNode);
	} else if (node.kind === ts.SyntaxKind.IndexedAccessType) {
		return convertIndexedAccessType(typeChecker, type as ts.IndexedAccessType);
	} else {
		console.log(ts.SyntaxKind[node.kind]);
		return { kind: "_unknown", value: typeChecker.typeToString(type) };
	}
};

let convertLiteralTypeNode = (
	_typeChecker: ts.TypeChecker,
	node: ts.LiteralTypeNode,
): LiteralType => {
	if (node.literal.kind === ts.SyntaxKind.StringLiteral) {
		return {
			kind: "literal",
			type: {
				kind: "string",
				value: node.literal.text,
			},
		};
	} else if (node.literal.kind === ts.SyntaxKind.NumericLiteral) {
		return {
			kind: "literal",
			type: {
				kind: "number",
				value: Number(node.literal.text),
			},
		};
	} else if (node.literal.kind === ts.SyntaxKind.TrueKeyword) {
		return {
			kind: "literal",
			type: {
				kind: "boolean",
				value: true,
			},
		};
	} else if (node.literal.kind === ts.SyntaxKind.FalseKeyword) {
		return {
			kind: "literal",
			type: {
				kind: "boolean",
				value: false,
			},
		};
	} else if (node.literal.kind === ts.SyntaxKind.NullKeyword) {
		return {
			kind: "literal",
			type: {
				kind: "null",
			},
		};
	} else {
		throw new Error("Unknown");
	}
};

let convertFunctionTypeNode = (
	typeChecker: ts.TypeChecker,
	node: ts.FunctionTypeNode,
): FunctionType => {
	let parameters = node.parameters?.map((parameter) =>
		convertParameterNode(typeChecker, parameter),
	);
	let typeParameters = node.typeParameters?.map((typeParameter) =>
		convertTypeParameterNode(typeChecker, typeParameter),
	);
	return {
		kind: "function",
		parameters: parameters ?? [],
		typeParameters: typeParameters ?? [],
		return: convertTypeNode(typeChecker, node.type),
	};
};

let convertFunctionType = (
	typeChecker: ts.TypeChecker,
	type: ts.Type,
): FunctionType => {
	let signature = type.getCallSignatures()[0]!;
	let parameters = signature.getParameters().map((parameter) => {
		let parameterType = typeChecker.getTypeOfSymbol(parameter);
		let declaration: ts.ParameterDeclaration | undefined =
			parameter.valueDeclaration as ts.ParameterDeclaration;
		let optional = false;
		if (declaration) {
			if (ts.isParameter(declaration) && declaration.questionToken) {
				optional = true;
			}
		}
		let comment = ts.displayPartsToString(
			parameter.getDocumentationComment(typeChecker),
		);
		return {
			name: parameter.getName(),
			type: declaration?.type
				? convertTypeNode(typeChecker, declaration.type)
				: convertType(typeChecker, parameterType),
			optional,
			comment,
		};
	});

	let typeParameters: Array<TypeParameter> = [];
	let callSignatureTypeParameters = signature.getTypeParameters();
	if (callSignatureTypeParameters) {
		typeParameters = callSignatureTypeParameters.map((typeParameter) =>
			convertTypeParameter(typeChecker, typeParameter),
		);
	}

	let declaration = signature.getDeclaration() as ts.SignatureDeclaration;
	let returnType: Type;
	let predicate = typeChecker.getTypePredicateOfSignature(signature);
	if (predicate) {
		returnType = convertTypePredicate(typeChecker, predicate);
	} else if (declaration.type) {
		returnType = convertTypeNode(typeChecker, declaration.type);
	} else {
		returnType = convertType(typeChecker, signature.getReturnType());
	}

	return {
		kind: "function",
		parameters,
		typeParameters,
		return: returnType,
	};
};

let convertTypePredicateNode = (
	typeChecker: ts.TypeChecker,
	node: ts.TypePredicateNode,
): PredicateType => {
	let asserts = node.assertsModifier !== undefined;
	return {
		kind: "predicate",
		name: node.parameterName.getText(),
		type: node.type ? convertTypeNode(typeChecker, node.type) : undefined,
		asserts,
	};
};

let convertTypePredicate = (
	typeChecker: ts.TypeChecker,
	type: ts.TypePredicate,
): PredicateType => {
	let asserts =
		type.kind === ts.TypePredicateKind.AssertsIdentifier ||
		type.kind === ts.TypePredicateKind.AssertsThis;
	return {
		kind: "predicate",
		name: type.parameterName ?? "this",
		type: type.type ? convertType(typeChecker, type.type) : undefined,
		asserts,
	};
};

let convertParameterNode = (
	typeChecker: ts.TypeChecker,
	node: ts.ParameterDeclaration,
): Parameter => {
	return {
		name: node.name.getText(),
		optional: node.questionToken ? true : false,
		type: node.type
			? convertTypeNode(typeChecker, node.type)
			: convertType(typeChecker, typeChecker.getTypeAtLocation(node)),
	};
};

let convertTypeParameterNode = (
	typeChecker: ts.TypeChecker,
	node: ts.TypeParameterDeclaration,
): TypeParameter => {
	return {
		name: node.name.getText(),
		constraint: node.constraint
			? convertTypeNode(typeChecker, node.constraint)
			: undefined,
		default: node.default
			? convertTypeNode(typeChecker, node.default)
			: undefined,
	};
};

let convertTypeParameter = (
	typeChecker: ts.TypeChecker,
	type: ts.Type,
): TypeParameter => {
	let constraint = type.getConstraint();
	let default_ = type.getDefault();
	return {
		name: type.symbol.getName(),
		constraint: constraint ? convertType(typeChecker, constraint) : undefined,
		default: default_ ? convertType(typeChecker, default_) : undefined,
	};
};

let convertConditionalTypeNode = (
	typeChecker: ts.TypeChecker,
	node: ts.ConditionalTypeNode,
): ConditionalType => {
	return {
		kind: "conditional",
		checkType: convertTypeNode(typeChecker, node.checkType),
		extendsType: convertTypeNode(typeChecker, node.extendsType),
		trueType: convertTypeNode(typeChecker, node.trueType),
		falseType: convertTypeNode(typeChecker, node.falseType),
	};
};

let convertInferTypeNode = (
	typeChecker: ts.TypeChecker,
	node: ts.InferTypeNode,
): InferType => {
	return {
		kind: "infer",
		typeParameter: convertTypeParameterNode(typeChecker, node.typeParameter),
	};
};

let convertMappedTypeNode = (
	typeChecker: ts.TypeChecker,
	node: ts.MappedTypeNode,
): MappedType => {
	return {
		kind: "mapped",
		type: convertTypeNode(typeChecker, node.type!),
		constraint: convertTypeNode(typeChecker, node.typeParameter.constraint!),
		typeParameterName: node.typeParameter.name.text,
		nameType: node.nameType
			? convertTypeNode(typeChecker, node.nameType)
			: undefined,
	};
};

let convertIndexedAccessTypeNode = (
	typeChecker: ts.TypeChecker,
	node: ts.IndexedAccessTypeNode,
): IndexedAccessType => {
	return {
		kind: "indexed_access",
		objectType: convertTypeNode(typeChecker, node.objectType),
		indexType: convertTypeNode(typeChecker, node.indexType),
	};
};

let convertIndexedAccessType = (
	typeChecker: ts.TypeChecker,
	type: ts.IndexedAccessType,
): IndexedAccessType => {
	return {
		kind: "indexed_access",
		objectType: convertType(typeChecker, type.objectType),
		indexType: convertType(typeChecker, type.indexType),
	};
};

let convertTemplateLiteralTypeNode = (
	typeChecker: ts.TypeChecker,
	node: ts.TemplateLiteralTypeNode,
): TemplateLiteralType => {
	return {
		kind: "template_literal",
		head: node.head.text,
		templateSpans: node.templateSpans.map((span) =>
			convertTemplateLiteralTypeSpan(typeChecker, span),
		),
	};
};

let convertTemplateLiteralTypeSpan = (
	typeChecker: ts.TypeChecker,
	node: ts.TemplateLiteralTypeSpan,
): TemplateLiteralTypeSpan => {
	return {
		type: convertTypeNode(typeChecker, node.type),
		literal: node.literal.text,
	};
};

let convertTypeQueryNode = (
	typeChecker: ts.TypeChecker,
	node: ts.TypeQueryNode,
): TypeQueryType => {
	let symbol = typeChecker.getSymbolAtLocation(node.exprName)!;
	symbol = getAliasedSymbolIfAliased(typeChecker, symbol);
	return {
		kind: "type_query",
		name: node.exprName.getText(),
		location: convertLocation(symbol.declarations![0]!),
	};
};

let convertTypeOperatorNode = (
	typeChecker: ts.TypeChecker,
	node: ts.TypeOperatorNode,
): TypeOperatorType => {
	return {
		kind: "type_operator",
		operator: operatorToName[node.operator],
		type: convertTypeNode(typeChecker, node.type),
	};
};

let keywordToName: { [key: number]: string } = {
	[ts.SyntaxKind.AnyKeyword]: "any",
	[ts.SyntaxKind.BigIntKeyword]: "bigint",
	[ts.SyntaxKind.BooleanKeyword]: "boolean",
	[ts.SyntaxKind.NeverKeyword]: "never",
	[ts.SyntaxKind.NumberKeyword]: "number",
	[ts.SyntaxKind.ObjectKeyword]: "object",
	[ts.SyntaxKind.StringKeyword]: "string",
	[ts.SyntaxKind.SymbolKeyword]: "symbol",
	[ts.SyntaxKind.UndefinedKeyword]: "undefined",
	[ts.SyntaxKind.UnknownKeyword]: "unknown",
	[ts.SyntaxKind.VoidKeyword]: "void",
};

let keywordSet = new Set([
	ts.SyntaxKind.AnyKeyword,
	ts.SyntaxKind.BigIntKeyword,
	ts.SyntaxKind.BooleanKeyword,
	ts.SyntaxKind.NeverKeyword,
	ts.SyntaxKind.NumberKeyword,
	ts.SyntaxKind.ObjectKeyword,
	ts.SyntaxKind.StringKeyword,
	ts.SyntaxKind.SymbolKeyword,
	ts.SyntaxKind.UndefinedKeyword,
	ts.SyntaxKind.UnknownKeyword,
	ts.SyntaxKind.VoidKeyword,
]);

let operatorToName = {
	[ts.SyntaxKind.KeyOfKeyword]: "keyof",
	[ts.SyntaxKind.UniqueKeyword]: "unique",
	[ts.SyntaxKind.ReadonlyKeyword]: "readonly",
};

let convertKeyword = (kind: ts.SyntaxKind): KeywordType => {
	return {
		kind: "keyword",
		name: keywordToName[kind] as KeywordType["name"],
	};
};

let convertArrayType = (
	typeChecker: ts.TypeChecker,
	type: ts.Type,
): ArrayType | ReferenceType => {
	if (type.symbol) {
		let symbol = getAliasedSymbolIfAliased(typeChecker, type.symbol);
		let typeArguments = typeChecker
			.getTypeArguments(type as ts.TypeReference)
			.map((typeArgument) => convertType(typeChecker, typeArgument));
		let declaration = symbol.declarations![0]!;
		return {
			kind: "reference",
			exported: isExported(declaration),
			location: convertLocation(declaration),
			name: symbol.getName(),
			typeArguments: typeArguments ?? [],
		};
	}
	return {
		kind: "array",
		type: convertType(
			typeChecker,
			typeChecker.getTypeArguments(type as ts.TypeReference)[0]!,
		),
	};
};

let convertArrayTypeNode = (
	typeChecker: ts.TypeChecker,
	node: ts.ArrayTypeNode,
): ArrayType => {
	return {
		kind: "array",
		type: convertTypeNode(typeChecker, node.elementType),
	};
};

let convertUnionType = (
	typeChecker: ts.TypeChecker,
	type: ts.UnionType,
): UnionType => {
	return {
		kind: "union",
		// types: type.types.map((type) => convertType(typeChecker, type)),
		types: type.types.map((type) => {
			return {
				kind: "_unknown",
				value: typeChecker.typeToString(type),
			};
		}),
	};
};

let convertUnionTypeNode = (
	typeChecker: ts.TypeChecker,
	node: ts.UnionTypeNode,
): UnionType => {
	return {
		kind: "union",
		types: node.types.map((node) => convertTypeNode(typeChecker, node)),
	};
};

let convertIntersectionType = (
	typeChecker: ts.TypeChecker,
	type: ts.IntersectionType,
): IntersectionType => {
	return {
		kind: "intersection",
		types: type.types.map((type) => convertType(typeChecker, type)),
	};
};

let convertIntersectionTypeNode = (
	typeChecker: ts.TypeChecker,
	node: ts.IntersectionTypeNode,
): IntersectionType => {
	return {
		kind: "intersection",
		types: node.types.map((node) => convertTypeNode(typeChecker, node)),
	};
};

let convertTupleType = (
	typeChecker: ts.TypeChecker,
	type: ts.TupleType,
): TupleType => {
	return {
		kind: "tuple",
		types: typeChecker
			.getTypeArguments(type)
			.map((type) => convertType(typeChecker, type)),
	};
};

let convertTupleTypeNode = (
	typeChecker: ts.TypeChecker,
	node: ts.TupleTypeNode,
): TupleType => {
	return {
		kind: "tuple",
		types: node.elements.map((node) => convertTypeNode(typeChecker, node)),
	};
};

let convertObjectTypeNode = (
	typeChecker: ts.TypeChecker,
	node: ts.TypeLiteralNode,
): Object => {
	let type = typeChecker.getTypeAtLocation(node);
	let properties = typeChecker.getPropertiesOfType(type).map((property) => {
		let type: Type;
		let valueDeclaration = property.valueDeclaration as ts.PropertySignature;
		if (valueDeclaration.type) {
			type = convertTypeNode(typeChecker, valueDeclaration.type);
		} else {
			type = convertType(typeChecker, typeChecker.getTypeOfSymbol(property));
		}
		return {
			name: property.getName(),
			type,
		};
	});

	let indexSignature: IndexSignature | undefined;
	let indexSymbol = type.symbol?.members?.get("__index" as ts.__String);
	if (indexSymbol) {
		let declaration =
			indexSymbol.declarations![0]! as ts.IndexSignatureDeclaration;
		let key = convertParameterNode(typeChecker, declaration.parameters[0]!);
		let type = convertTypeNode(typeChecker, declaration.type);
		indexSignature = {
			type,
			key,
		};
	}

	return {
		kind: "object",
		properties,
		indexSignature,
	};
};

let convertObjectType = (
	typeChecker: ts.TypeChecker,
	type: ts.Type,
): Object => {
	let properties = typeChecker.getPropertiesOfType(type).map((property) => {
		let type = convertType(typeChecker, typeChecker.getTypeOfSymbol(property));
		return {
			name: property.getName(),
			type,
		};
	});
	return {
		kind: "object",
		properties,
	};
};

let convertTypeReferenceTypeNode = (
	typeChecker: ts.TypeChecker,
	node: ts.TypeReferenceNode,
): ReferenceType => {
	let symbol = typeChecker.getSymbolAtLocation(node.typeName)!;
	let resolved = getAliasedSymbolIfAliased(typeChecker, symbol);
	let typeArguments = node.typeArguments?.map((typeArgument) =>
		convertTypeNode(typeChecker, typeArgument),
	);
	let declaration = resolved.declarations![0]!;
	return {
		kind: "reference",
		exported: isExported(declaration),
		location: convertLocation(declaration),
		name: node.typeName.getText(),
		typeArguments: typeArguments ?? [],
	};
};

let convertTypeReferenceType = (
	typeChecker: ts.TypeChecker,
	type: ts.Type,
): ReferenceType => {
	if (type.aliasSymbol) {
		let aliasSymbol = type.aliasSymbol;
		let typeArguments = type.aliasTypeArguments?.map((typeArgument) =>
			convertType(typeChecker, typeArgument),
		);
		let declaration = aliasSymbol.declarations![0]!;
		return {
			kind: "reference",
			name: aliasSymbol.getName(),
			location: convertLocation(declaration),
			exported: isExported(declaration),
			typeArguments: typeArguments ?? [],
		};
	} else {
		let typeArguments = typeChecker
			.getTypeArguments(type as ts.TypeReference)
			.map((typeArgument) => convertType(typeChecker, typeArgument));
		let symbol = type.symbol;
		let declaration = symbol.declarations![0]!;
		return {
			kind: "reference",
			name: symbol.getName(),
			location: convertLocation(declaration),
			typeArguments: typeArguments ?? [],
			exported: isExported(declaration),
		};
	}
};

let convertLocation = (node: ts.Node): Location => {
	let sourceFile = node.getSourceFile();
	let start = ts.getLineAndCharacterOfPosition(sourceFile, node.getStart());
	let end = ts.getLineAndCharacterOfPosition(sourceFile, node.getEnd());
	let module_ = typescript.moduleFromFileName(sourceFile.fileName);
	let docModule: DocModule;
	if (module_.kind == "library") {
		docModule = module_;
	} else if (module_.kind == "normal") {
		docModule = {
			kind: "normal",
			value: {
				modulePath: module_.value.modulePath,
				packageHash: syscall("package_hash_for_module", module_),
			},
		};
	} else {
		throw new Error("Invalid module kind.");
	}
	return {
		module: docModule,
		range: {
			start,
			end,
		},
	};
};

let convertComment = (
	typeChecker: ts.TypeChecker,
	symbol: ts.Symbol,
): Comment => {
	let summary = ts.displayPartsToString(
		symbol.getDocumentationComment(typeChecker),
	);
	let tags = symbol.getJsDocTags(typeChecker).map((tag) => {
		return { name: tag.name, comment: ts.displayPartsToString(tag.text) };
	});
	return {
		summary,
		tags,
	};
};

function getAliasedSymbolIfAliased(
	typeChecker: ts.TypeChecker,
	symbol: ts.Symbol,
) {
	if ((symbol.flags & ts.SymbolFlags.Alias) !== 0) {
		return typeChecker.getAliasedSymbol(symbol);
	}
	return symbol;
}

let stringify = (doc: Doc) => {
	let exports_: Array<string> = [];
	for (let export_ of doc.exports) {
		exports_.push(stringifyExport(export_));
	}
	return exports_.join("\n");
};

let stringifyExport = (export_: Export) => {
	switch (export_.kind) {
		case "variable":
			return stringifyVariableExport(export_);
		case "type":
			return stringifyTypeExport(export_);
		default:
			throw new Error();
	}
};

let stringifyType = (type: Type) => {
	switch (type.kind) {
		case "literal":
			return stringifyLiteralType(type);
		case "keyword":
			return stringifyKeywordType(type);
		case "reference":
			return stringifyReferenceType(type);
		case "union":
			return stringifyUnionType(type);
		case "intersection":
			return stringifyIntersectionType(type);
		case "tuple":
			return stringifyTupleType(type);
		case "array":
			return stringifyArrayType(type);
		case "object":
			return stringifyTypeLiteralType(type);
		case "function":
			return stringifyFunctionType(type);
		case "infer":
			return stringifyInferType(type);
		case "conditional":
			return stringifyConditionalType(type);
		case "mapped":
			return stringifyMappedType(type);
		case "template_literal":
			return stringifyTemplateLiteralType(type);
		case "indexed_access":
			return stringifyIndexedAccessType(type);
		case "type_query":
			return stringifyTypeQueryType(type);
		case "type_operator":
			return stringifyTypeOperatorType(type);
		case "predicate":
			return stringifyPredicateType(type);
		case "_unknown":
			return type.value;
		default:
			throw new Error();
	}
};

let stringifyTypeExport = (value: TypeExport) => {
	let typeParameters = "";
	if (value.typeParameters.length > 0) {
		typeParameters =
			"<" + value.typeParameters.map(stringifyTypeParameter).join(", ") + ">";
	}
	return `${value.name}${typeParameters}: ${stringifyType(value.type)}`;
};

let stringifyVariableExport = (value: VariableExport) => {
	return `${value.name}: ${stringifyType(value.type)}`;
};

let stringifyLiteralType = (value: LiteralType) => {
	switch (value.type.kind) {
		case "string":
			return stringifyStringLiteralType(value.type);
		case "number":
			return stringifyNumberLiteralType(value.type);
		case "null":
			return stringifyNullLiteralType(value.type);
		case "boolean":
			return stringifyBooleanLiteralType(value.type);
		case "unknown":
			return stringifyUnknownLiteralType(value.type);
		default:
			throw new Error();
	}
};

let stringifyStringLiteralType = (value: StringLiteralType) => {
	return `"${value.value}"`;
};

let stringifyNumberLiteralType = (value: NumberLiteralType) => {
	return value.value.toString();
};

let stringifyNullLiteralType = (value: NullLiteralType) => {
	return "null";
};

let stringifyBooleanLiteralType = (value: BooleanLiteralType) => {
	return value.value.toString();
};

let stringifyUnknownLiteralType = (value: UnknownLiteralType) => {
	return value.value.toString();
};

let stringifyKeywordType = (value: KeywordType) => {
	return value.name;
};

let stringifyReferenceType = (value: ReferenceType) => {
	let typeArguments = "";
	if (value.typeArguments.length > 0) {
		typeArguments =
			"<" + value.typeArguments.map(stringifyType).join(",") + ">";
	}
	return `${value.name}${typeArguments}`;
};

let stringifyUnionType = (value: UnionType): string => {
	return value.types.map(stringifyType).join(" | ");
};

let stringifyTupleType = (value: TupleType): string => {
	return `[${value.types.map(stringifyType).join(", ")}]`;
};

let stringifyIntersectionType = (value: IntersectionType): string => {
	return value.types.map(stringifyType).join(" & ");
};

let stringifyArrayType = (value: ArrayType): string => {
	return `Array<${stringifyType(value.type)}>`;
};

let stringifyTypeLiteralType = (value: Object): string => {
	let indexSignature = "";
	if (value.indexSignature) {
		indexSignature = stringifyIndexSignatureType(value.indexSignature);
	}
	return `{${indexSignature}${value.properties
		.map(stringifyPropertyType)
		.join(", ")}}`;
};

let stringifyIndexSignatureType = (value: IndexSignature): string => {
	return `[${stringifyParameter(value.key)}]: ${stringifyType(value.type)}]`;
};

let stringifyPropertyType = (value: PropertyType): string => {
	return `${value.name}: ${stringifyType(value.type)}`;
};

let stringifyFunctionType = (value: FunctionType): string => {
	let typeParameters = "";
	if (value.typeParameters.length > 0) {
		typeParameters =
			"<" + value.typeParameters.map(stringifyTypeParameter).join(", ") + ">";
	}
	let parameters = "";
	if (value.parameters.length > 0) {
		parameters = value.parameters.map(stringifyParameter).join(", ");
	}
	return `${typeParameters}(${parameters}) => ${stringifyType(value.return)}`;
};

let stringifyConditionalType = (value: ConditionalType): string => {
	return `${stringifyType(value.checkType)} extends ${stringifyType(
		value.extendsType,
	)} ? : ${stringifyType(value.trueType)} : ${stringifyType(value.falseType)}`;
};

let stringifyInferType = (value: InferType): string => {
	return `infer ${stringifyTypeParameter(value.typeParameter)}`;
};

let stringifyMappedType = (value: MappedType): string => {
	let nameType = "";
	if (value.nameType) {
		nameType = ` as ${stringifyType(value.nameType)}`;
	}
	return `[${value.typeParameterName} in ${stringifyType(
		value.constraint,
	)}${nameType}]: ${stringifyType(value.type)}`;
};

let stringifyIndexedAccessType = (value: IndexedAccessType): string => {
	return `${stringifyType(value.objectType)}[${stringifyType(
		value.indexType,
	)}]`;
};

let stringifyTemplateLiteralType = (value: TemplateLiteralType): string => {
	let rest = "";
	if (value.templateSpans.length > 0) {
		rest = value.templateSpans.map(stringifyTemplateLiteralTypeSpan).join("");
	}
	return `\`${value.head}${rest}\``;
};

let stringifyTemplateLiteralTypeSpan = (
	value: TemplateLiteralTypeSpan,
): string => {
	return `\$\{${stringifyType(value.type)}\}${value.literal}`;
};

let stringifyTypeQueryType = (value: TypeQueryType): string => {
	return `typeof ${value.name}`;
};

let stringifyTypeOperatorType = (value: TypeOperatorType): string => {
	return `${value.operator} ${stringifyType(value.type)}`;
};

let stringifyPredicateType = (value: PredicateType): string => {
	let asserts = value.asserts ? "asserts " : "";
	let target = "";
	if (value.type) {
		target = `is ${stringifyType(value.type)}`;
	}
	return `${asserts}${value.name}${target}`;
};

let stringifyParameter = (value: Parameter): string => {
	return `${value.name}${value.optional ? "?" : ""}: ${stringifyType(
		value.type,
	)}`;
};

let stringifyTypeParameter = (value: TypeParameter): string => {
	let constraint = "";
	if (value.constraint) {
		constraint = " extends " + stringifyType(value.constraint);
	}
	let default_ = "";
	if (value.default) {
		default_ = " = " + stringifyType(value.default);
	}
	return `${value.name}${constraint}${default_}`;
};

let toJSON = (doc: Doc) => {
	return JSON.stringify(doc);
};

let isExported = (node: ts.Node): boolean => {
	return (
		(ts.getCombinedModifierFlags(node as ts.Declaration) &
			ts.ModifierFlags.Export) !==
		0
	);
};
export let handle = (request: Request): Response => {
	// Create the program and type checker.
	let program = ts.createProgram({
		rootNames: [typescript.fileNameFromModule(request.module)],
		options: compilerOptions,
		host,
	});
	let typeChecker = program.getTypeChecker();

	// Get the module's exports.
	let sourceFile = program.getSourceFile(
		typescript.fileNameFromModule(request.module),
	)!;
	let symbol = typeChecker.getSymbolAtLocation(sourceFile)!;
	let moduleExports = typeChecker.getExportsOfModule(symbol);

	// Convert the exports.
	let exports: Array<Export> = [];
	for (let moduleExport of moduleExports) {
		exports.push(...convertSymbol(typeChecker, moduleExport));
	}

	// Create the doc.
	let doc = {
		exports,
	};

	return {
		doc,
	};
};

declare module "typescript" {
	interface TypeChecker {
		// https://github.com/microsoft/TypeScript/blob/v4.7.2/src/compiler/types.ts#L4188
		getTypeOfSymbol(symbol: Symbol): Type;
	}
}
