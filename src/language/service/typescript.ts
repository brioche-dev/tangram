import * as ts from "typescript";

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
		return "tangram://lib/tangram.d.ts";
	},

	getDefaultLibLocation: () => {
		return "tangram://lib";
	},

	getNewLine: () => {
		return "\n";
	},

	getScriptFileNames: () => {
		return syscall("get_documents");
	},

	getScriptSnapshot: (fileName) => {
		let text;
		try {
			text = syscall("load_module", fileName);
		} catch {
			return undefined;
		}
		return ts.ScriptSnapshot.fromString(text);
	},

	getScriptVersion: (fileName) => {
		return syscall("get_module_version", fileName);
	},

	getSourceFile: (fileName, languageVersion) => {
		let text;
		try {
			text = syscall("load_module", fileName);
		} catch {
			return undefined;
		}
		let sourceFile = ts.createSourceFile(fileName, text, languageVersion);
		return sourceFile;
	},

	hasInvalidatedResolutions: (_fileName) => {
		return false;
	},

	readFile: (f) => {
		throw new Error("Unimplemented.");
	},

	resolveModuleNames: (specifiers, referrer) => {
		return specifiers.map((specifier) => {
			let resolvedFileName;
			try {
				resolvedFileName = syscall("resolve_module", specifier, referrer);
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
