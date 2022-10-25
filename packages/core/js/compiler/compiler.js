globalThis.handle = ({ type, request }) => {
	switch (type) {
		case "check": {
			let diagnostics = check(request);
			return {
				type: "check",
				response: { diagnostics },
			};
		}
		case "get_diagnostics": {
			let diagnostics = getDiagnostics(request);
			return {
				type: "get_diagnostics",
				response: { diagnostics },
			};
		}
		default: {
			throw new Error(`Unknown request type "${type}".`);
		}
	}
};

let compilerOptions = {
	maxNodeModuleJsDepth: 0,
	module: ts.ModuleKind.ESNext,
	noEmit: true,
	strict: true,
	target: ts.ScriptTarget.ESNext,
};

let host = {
	getCompilationSettings: () => compilerOptions,
	getCanonicalFileName: (fileName) => {
		return fileName;
	},
	getCurrentDirectory: () => {
		return undefined;
	},
	getDefaultLibFileName: () => "/__tangram_typescript_lib__/lib.d.ts",
	getNewLine: () => "\n",
	getScriptFileNames: () => {
		return Deno.core.opSync("op_tg_documents");
	},
	getScriptSnapshot: (fileName) => {
		let source = Deno.core.opSync("op_tg_load", fileName);
		return ts.ScriptSnapshot.fromString(source);
	},
	getScriptVersion: (fileName) => {
		return Deno.core.opSync("op_tg_version", fileName);
	},
	getSourceFile: (fileName, languageVersion, _onError) => {
		let source = Deno.core.opSync("op_tg_load", fileName);
		return ts.createSourceFile(fileName, source, languageVersion);
	},
	resolveModuleNames: (specifiers, referrer) => {
		return specifiers.map((specifier) => {
			let path = Deno.core.opSync("op_tg_resolve", specifier, referrer);
			return {
				resolvedFileName: path,
				extension: ".ts",
			};
		});
	},
	useCaseSensitiveFileNames: () => true,
};

let languageService = ts.createLanguageService(host);

let check = ({ paths }) => {
	let program = ts.createProgram([...paths], compilerOptions, host);
	let diagnostics = [
		...program.getConfigFileParsingDiagnostics(),
		...program.getOptionsDiagnostics(),
		...program.getGlobalDiagnostics(),
		...program.getDeclarationDiagnostics(),
		...program.getSyntacticDiagnostics(),
		...program.getSemanticDiagnostics(),
	];
	diagnostics = convertDiagnostics(diagnostics);
	return diagnostics;
};

let getDiagnostics = () => {
	let paths = Deno.core.opSync("op_tg_documents");
	let diagnostics = {};
	for (let path of paths) {
		diagnostics[path] = [
			...languageService.getSyntacticDiagnostics(path),
			...languageService.getSemanticDiagnostics(path),
			...languageService.getSuggestionDiagnostics(path),
		].map(convertDiagnostic);
	}
	return diagnostics;
};

/** Convert TypeScript diagnostics to Tangram diagnostics. */
let convertDiagnostics = (diagnostics) => {
	let output = {};

	for (diagnostic of diagnostics) {
		// Ignore diagnostics that do not have a file.
		if (!diagnostic.file) {
			continue;
		}

		// Add an entry for this diagnostic's file in the output if necessary.
		let path = diagnostic.file.fileName;
		if (output[path] === undefined) {
			output[path] = [];
		}

		// Add the diagnostic to the output.
		output[path].push(convertDiagnostic(diagnostic));
	}

	return output;
};

let convertDiagnostic = (diagnostic) => {
	// Get the diagnostic's path.
	let path = diagnostic.file.fileName;

	// Get the diagnostic's range.
	let start = ts.getLineAndCharacterOfPosition(
		diagnostic.file,
		diagnostic.start,
	);
	let end = ts.getLineAndCharacterOfPosition(
		diagnostic.file,
		diagnostic.start + diagnostic.length,
	);
	let range = { start, end };

	let location = {
		path,
		range,
	};

	// Get the diagnostic's message.
	let message = ts.flattenDiagnosticMessageText(diagnostic.messageText, "\n");

	return {
		location,
		message,
	};
};

globalThis.console = {
	log: (...args) => {
		let string = args.map((arg) => print(arg)).join(" ");
		Deno.core.opSync("op_tg_print", string);
	},
	error: (...args) => {
		let string = args.map((arg) => print(arg)).join(" ");
		Deno.core.opSync("op_tg_print", string);
	},
};

let print = (value) => {
	if (value === undefined) {
		return "undefined";
	} else if (value === null) {
		return "null";
	} else if (Array.isArray(value)) {
		return `[${value.map(print).join(", ")}]`;
	} else if (value instanceof Error) {
		return value.stack;
	} else if (value instanceof Promise) {
		return "Promise";
	} else if (typeof value === "object") {
		let constructorName = "";
		if (value.constructor.name !== "Object") {
			constructorName = `${value.constructor.name} `;
		}
		let entries = Object.entries(value).map(
			([key, value]) => `${key}: ${print(value)}`,
		);
		return `${constructorName}{ ${entries.join(", ")} }`;
	} else if (typeof value === "function") {
		return `[Function: ${value.name || "(anonymous)"}]`;
	} else {
		return JSON.stringify(value);
	}
};
