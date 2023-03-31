import * as syscall from "./syscall.ts";
import { ModuleIdentifier } from "./syscall.ts";
import ts from "typescript";

// Create the TypeScript compiler options.
export let compilerOptions: ts.CompilerOptions = {
	isolatedModules: true,
	module: ts.ModuleKind.ESNext,
	noEmit: true,
	skipLibCheck: true,
	strict: true,
	target: ts.ScriptTarget.ESNext,
};

// Create the host implementation for the TypeScript language service and compiler.
export let host: ts.LanguageServiceHost & ts.CompilerHost = {
	fileExists: (f) => {
		return false;
	},

	getCompilationSettings: () => {
		return compilerOptions;
	},

	getCanonicalFileName: (fileName) => {
		return fileName;
	},

	getCurrentDirectory: () => {
		return "/";
	},

	getDefaultLibFileName: () => {
		return "/lib/tangram.d.ts";
	},

	getDefaultLibLocation: () => {
		return "/lib";
	},

	getNewLine: () => {
		return "\n";
	},

	getScriptFileNames: () => {
		return syscall.getDocuments().map(fileNameFromModuleIdentifier);
	},

	getScriptSnapshot: (fileName) => {
		let text;
		try {
			text = syscall.loadModule(moduleIdentifierFromFileName(fileName));
		} catch {
			return undefined;
		}
		return ts.ScriptSnapshot.fromString(text);
	},

	getScriptVersion: (fileName) => {
		return syscall.getModuleVersion(moduleIdentifierFromFileName(fileName));
	},

	getSourceFile: (fileName, languageVersion) => {
		let text;
		try {
			text = syscall.loadModule(moduleIdentifierFromFileName(fileName));
		} catch {
			return undefined;
		}
		let sourceFile = ts.createSourceFile(fileName, text, languageVersion);
		return sourceFile;
	},

	hasInvalidatedResolutions: (_fileName) => {
		return false;
	},

	readFile: () => {
		throw new Error("Unimplemented.");
	},

	resolveModuleNames: (specifiers, referrer) => {
		return specifiers.map((specifier) => {
			let resolvedFileName;
			try {
				resolvedFileName = fileNameFromModuleIdentifier(
					syscall.resolveModule(
						specifier,
						moduleIdentifierFromFileName(referrer),
					),
				);
			} catch {
				return undefined;
			}
			return { resolvedFileName, extension: ts.Extension.Ts };
		});
	},

	useCaseSensitiveFileNames: () => {
		return true;
	},

	writeFile: () => {
		throw new Error("Unimplemented.");
	},
};

// Create the TypeScript language service.
export let languageService = ts.createLanguageService(host);

/** Convert a module identifier to a TypeScript file name. */
export let fileNameFromModuleIdentifier = (
	moduleIdentifier: ModuleIdentifier,
): string => {
	let data = syscall.encodeHex(
		syscall.encodeUtf8(JSON.stringify(moduleIdentifier)),
	);
	let fileName = `/${data}.ts`;
	return fileName;
};

/** Convert a TypeScript file name to a module identifier. */
export let moduleIdentifierFromFileName = (
	fileName: string,
): ModuleIdentifier => {
	let moduleIdentifier;
	if (fileName.startsWith("/lib/")) {
		let path = fileName.slice(5);
		moduleIdentifier = { source: { kind: "lib" as const }, path };
	} else {
		let data = fileName.slice(1, -3);
		moduleIdentifier = JSON.parse(syscall.decodeUtf8(syscall.decodeHex(data)));
	}
	return moduleIdentifier;
};
