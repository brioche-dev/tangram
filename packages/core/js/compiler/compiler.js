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

// Create the TypeScript compiler options.
let compilerOptions = {
	allowNonTsExtensions: true,
	lib: ["lib.esnext.full.d.ts", "lib.tangram.ns.d.ts"],
	maxNodeModuleJsDepth: 0,
	module: ts.ModuleKind.ESNext,
	noEmit: true,
	strict: true,
	target: ts.ScriptTarget.ESNext,
};

// Create the host implementation for the TypeScript compiler.
let host = {
	getCompilationSettings: () => {
		return compilerOptions;
	},
	getCanonicalFileName: (fileName) => {
		return fileName;
	},
	getCurrentDirectory: () => {
		return undefined;
	},
	getDefaultLibFileName: () => {
		return "tangram-typescript-lib:///lib.esnext.full.d.ts";
	},
	getDefaultLibLocation: () => {
		return "tangram-typescript-lib:///";
	},
	getNewLine: () => {
		return "\n";
	},
	getScriptFileNames: () => {
		return syscall(Syscall.OpenedFiles);
	},
	getScriptSnapshot: (fileName) => {
		let { text } = syscall(Syscall.Load, fileName);
		return ts.ScriptSnapshot.fromString(text);
	},
	getScriptVersion: (fileName) => {
		return syscall(Syscall.Version, fileName);
	},
	getSourceFile: (fileName, languageVersion, _onError) => {
		let { text, version } = syscall(Syscall.Load, fileName);
		let sourceFile = ts.createSourceFile(fileName, text, languageVersion);
		sourceFile.moduleName = fileName;
		sourceFile.version = version;
		return sourceFile;
	},
	resolveModuleNames: (specifiers, referrer) => {
		return specifiers.map((specifier) => {
			let resolvedFileName = syscall(Syscall.Resolve, specifier, referrer);
			return {
				resolvedFileName: resolvedFileName,
				extension: ".ts",
			};
		});
	},
	useCaseSensitiveFileNames: () => {
		return true;
	},
	readFile: (fileName) => {
		return undefined;
	},
	fileExists: (fileName) => {
		return false;
	},
};

// // Add our libs to the private libs field.
// ts.libs.push("tangram.ns");
// ts.libMap.set("tangram.ns", "lib.tangram.ns.d.ts");

// Create the TypeScript language service.
let languageService = ts.createLanguageService(host);

let check = ({ urls }) => {
	let program = ts.createIncrementalProgram({
		rootNames: [...urls],
		options: compilerOptions,
		host,
	});
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
	let urls = syscall(Syscall.OpenedFiles);
	let diagnostics = {};
	for (let url of urls) {
		diagnostics[url] = [
			...languageService.getSyntacticDiagnostics(url),
			...languageService.getSemanticDiagnostics(url),
			...languageService.getSuggestionDiagnostics(url),
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
		let url = diagnostic.file.fileName;
		if (output[url] === undefined) {
			output[url] = [];
		}

		// Add the diagnostic to the output.
		output[url].push(convertDiagnostic(diagnostic));
	}

	return output;
};

/** Convert a TypeScript diagnostic to a Tangram diagnostic. */
let convertDiagnostic = (diagnostic) => {
	// Get the diagnostic's location.
	let location = null;
	if (diagnostic.file) {
		// Get the diagnostic's URL.
		let url = diagnostic.file.fileName;

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

		location = {
			url,
			range,
		};
	}

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
		syscall(Syscall.Print, string);
	},
	error: (...args) => {
		let string = args.map((arg) => print(arg)).join(" ");
		syscall(Syscall.Print, string);
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

let Syscall = {
	Load: "load",
	OpenedFiles: "opened_files",
	Print: "print",
	Resolve: "resolve",
	Version: "version",
};

let syscall = (syscall, ...args) => {
	let opName = "op_tg_" + syscall;
	switch (syscall) {
		case Syscall.Load:
			return Deno.core.opSync(opName, ...args);
		case Syscall.OpenedFiles:
			return Deno.core.opSync(opName, ...args);
		case Syscall.Print:
			return Deno.core.opSync(opName, ...args);
		case Syscall.Resolve:
			return Deno.core.opSync(opName, ...args);
		case Syscall.Version:
			return Deno.core.opSync(opName, ...args);
	}
};
