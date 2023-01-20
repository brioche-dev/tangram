import * as ts from "typescript";

// Create the TypeScript compiler options.
export let compilerOptions = {
	allowNonTsExtensions: true,
	isolatedModules: true,
	lib: ["lib.esnext.d.ts", "global.d.ts"],
	maxNodeModuleJsDepth: 0,
	module: ts.ModuleKind.ESNext,
	noEmit: true,
	strict: true,
	target: ts.ScriptTarget.ESNext,
};

// Create the host implementation for the TypeScript language service and compiler.
export let host: ts.LanguageServiceHost & ts.CompilerHost = {
	getCompilationSettings: () => {
		return compilerOptions;
	},

	getCanonicalFileName: (fileName) => {
		return fileName;
	},

	getCurrentDirectory: () => {
		return "";
	},

	getDefaultLibFileName: () => {
		return "tangram-internal://lib/lib.esnext.d.ts";
	},

	getDefaultLibLocation: () => {
		return "tangram-internal://lib";
	},

	getNewLine: () => {
		return "\n";
	},

	getScriptFileNames: () => {
		return syscall("opened_files");
	},

	getScriptSnapshot: (fileName) => {
		let result;
		try {
			result = syscall("load", fileName);
		} catch {
			return undefined;
		}
		let { text } = result;
		return ts.ScriptSnapshot.fromString(text);
	},

	getScriptVersion: (fileName) => {
		return syscall("version", fileName);
	},

	getSourceFile: (fileName, languageVersion) => {
		let result;
		try {
			result = syscall("load", fileName);
		} catch {
			return undefined;
		}
		let { text } = result;
		let sourceFile = ts.createSourceFile(fileName, text, languageVersion);
		return sourceFile;
	},

	resolveModuleNames: (specifiers, referrer) => {
		return specifiers.map((specifier) => {
			let resolvedFileName;
			try {
				resolvedFileName = syscall("resolve", specifier, referrer);
			} catch {
				return undefined;
			}
			return { resolvedFileName, extension: ".ts" };
		});
	},

	useCaseSensitiveFileNames: () => {
		return true;
	},

	readFile: () => {
		return undefined;
	},

	fileExists: () => {
		return false;
	},

	writeFile: () => {
		throw new Error("Unimplemented.");
	},
};

// Create the TypeScript language service.
export let languageService = ts.createLanguageService(host);
