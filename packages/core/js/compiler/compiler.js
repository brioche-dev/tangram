/**
 * Handle a request from the embedder.
 */
async function handle({ type, request }) {
	if (type === "check") {
		let diagnostics = check(request.fileNames);
		return {
			type: "check",
			response: { diagnostics },
		};
	} else {
		throw new Error(`Unknown request type: ${type}`);
	}
}

globalThis.console = {
	log(...args) {
		Deno.core.opSync("op_tg_console_log", args);
	},
	error(...args) {
		Deno.core.opSync("op_tg_console_error", args);
	},
};

let compilerOptions = {
	noEmitOnError: true,
	noImplicitAny: true,
	noEmit: true,
	noLib: true, // Disable the default `lib.js`, we'll pass it ourselves explicitly.
	maxNodeModuleJsDepth: 0, // Don't resolve `import "file.js"` in node_modules
	target: ts.ScriptTarget.ES2022,
	module: ts.ModuleKind.CommonJS,
};

function tgReadFile(fileName) {
	let result = Deno.core.opSync("op_tg_read_file", fileName);
	if (typeof result == "string") {
		return result;
	} else {
		return undefined;
	}
}

function tgFileExists(fileName) {
	return Deno.core.opSync("op_tg_file_exists", fileName);
}

function tgGetSourceFile(fileName, languageVersion, onError) {
	const sourceText = tgReadFile(fileName);
	return sourceText !== undefined
		? ts.createSourceFile(fileName, sourceText, languageVersion)
		: undefined;
}

function tgResolveModuleNames(moduleNames, containingFile) {
	const resolvedModules = [];
	for (const moduleName of moduleNames) {
		// Call out to the host to resolve the module name.
		try {
			let resolved = Deno.core.opSync(
				"op_tg_resolve",
				containingFile,
				moduleName,
			);
			resolvedModules.push(resolved);
		} catch (e) {
			console.error(`Error resolving '${moduleName}': ${e.message}`);
			resolvedModules.push(undefined);
		}
	}
	return resolvedModules;
}

function check(fileNames) {
	if (!Array.isArray(fileNames)) {
		throw new Error(
			"check() expects an array of filenames as its first parameter",
		);
	}

	let host = {
		getDefaultLibFileName: () => "lib.d.ts",

		// Use Unix conventions for typechecking.
		getNewLine: () => "\n",

		// Use case-sensitive filenames
		getCanonicalFileName: (fileName) => fileName,
		useCaseSensitiveFileNames: () => true,

		// Use the Tangram VFS
		getCurrentDirectory: () => ".", // Not necessary
		readFile: tgReadFile,
		getSourceFile: tgGetSourceFile,
		resolveModuleNames: tgResolveModuleNames,
	};

	let program = ts.createProgram(
		["/__tangram__/internal/environment.d.ts", ...fileNames],
		compilerOptions,
		host,
	);
	let emitResult = program.emit();

	let allDiagnostics = ts
		.getPreEmitDiagnostics(program)
		.concat(emitResult.diagnostics);

	// Extract data from diagnostics before returning to Rust
	return allDiagnostics.map(exportDiagnostic);
}

/** Convert a TypeScript diagnostic into the format we're expecting in the embedding Rust code. */
function exportDiagnostic(diagnostic) {
	if (diagnostic.file) {
		let { line, character } = ts.getLineAndCharacterOfPosition(
			diagnostic.file,
			diagnostic.start,
		);
		let message = ts.flattenDiagnosticMessageText(diagnostic.messageText, "\n");

		return {
			kind: "File",
			file_name: diagnostic.file.fileName,
			line: line + 1,
			col: character + 1,
			message,
		};
	} else {
		return {
			kind: "Other",
			message: ts.flattenDiagnosticMessageText(diagnostic.messageText, "\n"),
		};
	}
}

function createService() {
	let serviceHost = {
		getCurrentDirectory: () => ".",
		getCompilationSettings: () => compilerOptions,
		getDefaultLibFileName: () => "lib.d.ts",

		// This should return the set of files the LSP considers "currently open".
		getScriptFileNames: () => [
			"/__tangram__/internal/environment.d.ts",
			...globalThis.openFiles,
		],

		// Use the Tangram module resolver.
		resolveModuleNames: tgResolveModuleNames,

		// Track versions for scripts.
		getScriptVersion: (fileName) => false, // TODO: do we need this to cache-invalidate?
		getScriptSnapshot: (fileName) => {
			console.log("getScriptSnapshot", fileName);
			const sourceText = tgReadFile(fileName);
			return sourceText !== undefined
				? ts.ScriptSnapshot.fromString(sourceText)
				: undefined;
		},

		readFile: tgReadFile,
	};

	return ts.createLanguageService(serviceHost, ts.createDocumentRegistry());
}

function getDiagnosticsForFile(fileName) {
	// TypeScript needs to consider this file as "open" while getting its diagnostics.
	globalThis.openFiles = [fileName];

	let diagnostics = [
		...globalThis.compilerService.getSyntacticDiagnostics(fileName),
		...globalThis.compilerService.getSemanticDiagnostics(fileName),
	];

	globalThis.openFiles = [];

	return diagnostics.map(exportDiagnostic);
}

// Create the compiler host at build-time, when the v8 snapshot is populated.
globalThis.compilerService = createService();
globalThis.openFiles = [];
