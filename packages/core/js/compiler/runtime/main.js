globalThis.handle = ({ type, request }) => {
	switch (type) {
		case "check": {
			return check(request);
		}
		case "find_rename_locations": {
			return findRenameLocations(request);
		}
		case "get_diagnostics": {
			return getDiagnostics(request);
		}
		case "get_hover": {
			return hover(request);
		}
		case "get_references": {
			return getReferences(request);
		}
		case "goto_definition": {
			return gotoDefinition(request);
		}
		case "completion": {
			return completion(request);
		}
		case "transpile": {
			return transpile(request);
		}
		default: {
			throw new Error(`Unknown request type "${type}".`);
		}
	}
};

// Create the TypeScript compiler options.
let compilerOptions = {
	allowNonTsExtensions: true,
	isolatedModules: true,
	lib: ["lib.esnext.d.ts", "lib.tangram.d.ts"],
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
		return "tangram-internal://lib/lib.esnext.d.ts";
	},

	getDefaultLibLocation: () => {
		return "tangram-internal://lib/";
	},

	getNewLine: () => {
		return "\n";
	},

	getScriptFileNames: () => {
		return syscall(Syscall.OpenedFiles);
	},

	getScriptSnapshot: (fileName) => {
		let result;
		try {
			result = syscall(Syscall.Load, fileName);
		} catch {
			return undefined;
		}
		let { text } = result;
		return ts.ScriptSnapshot.fromString(text);
	},

	getScriptVersion: (fileName) => {
		return syscall(Syscall.Version, fileName);
	},

	getSourceFile: (fileName, languageVersion) => {
		let result;
		try {
			result = syscall(Syscall.Load, fileName);
		} catch {
			return undefined;
		}
		let { text, version } = result;
		let sourceFile = ts.createSourceFile(fileName, text, languageVersion);
		sourceFile.version = version;
		return sourceFile;
	},

	resolveModuleNames: (specifiers, referrer) => {
		return specifiers.map((specifier) => {
			let resolvedFileName;
			try {
				resolvedFileName = syscall(Syscall.Resolve, specifier, referrer);
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
let languageService = ts.createLanguageService(host);

let check = (request) => {
	// Create a typescript program.
	let program = ts.createIncrementalProgram({
		rootNames: [...request.urls],
		options: compilerOptions,
		host,
	});

	// Get the diagnostics and convert them.
	let diagnostics = convertDiagnostics([
		...program.getConfigFileParsingDiagnostics(),
		...program.getOptionsDiagnostics(),
		...program.getGlobalDiagnostics(),
		...program.getDeclarationDiagnostics(),
		...program.getSyntacticDiagnostics(),
		...program.getSemanticDiagnostics(),
	]);

	return {
		type: "check",
		response: { diagnostics },
	};
};

let getDiagnostics = (_request) => {
	// Get the list of opened files.
	let urls = syscall(Syscall.OpenedFiles);

	// Collect the diagnostics for each opened file.
	let diagnostics = {};
	let ignored = {};
	for (let url of urls) {
		diagnostics[url] = [
			...languageService.getSyntacticDiagnostics(url),
			...languageService.getSemanticDiagnostics(url),
			...languageService.getSuggestionDiagnostics(url),
		].map(convertDiagnostic);
	}

	return {
		type: "get_diagnostics",
		response: { diagnostics, ignored },
	};
};

/** Convert TypeScript diagnostics to Tangram diagnostics. */
let convertDiagnostics = (diagnostics) => {
	let output = {};

	for (let diagnostic of diagnostics) {
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

// TS2691: An import path cannot end with a '.ts' extension. Consider importing 'bad-module' instead.
const TS2691 = 2691;

// TS2792: Cannot find module. Did you mean to set the 'moduleResolution' option to 'node', or to add aliases to the 'paths' option?
const TS2792 = 2792;

/** Convert a TypeScript diagnostic to a Tangram diagnostic. */
let convertDiagnostic = (diagnostic) => {
	// Get the diagnostic's URL.
	let url = diagnostic.file.fileName;

	// Get the diagnostic's location.
	let location = null;
	if (diagnostic.file) {
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

	// Convert the diagnostic's severity.
	let severity;
	switch (diagnostic.category) {
		case ts.DiagnosticCategory.Warning: {
			severity = "warning";
			break;
		}
		case ts.DiagnosticCategory.Error: {
			severity = "error";
			break;
		}
		case ts.DiagnosticCategory.Suggestion: {
			severity = "hint";
			break;
		}
		case ts.DiagnosticCategory.Message: {
			severity = "information";
			break;
		}
		default: {
			throw new Error("Unknown diagnostic category.");
		}
	}

	let message;
	// Map diagnostics for '.ts' extensions to bad Tangram url import errors instead.
	if (diagnostic.code === TS2691) {
		message = "Could not load dependency.";
	} else if (diagnostic.code === TS2792) {
		message = "Could not load dependency.";
	} else {
		// Get the diagnostic's message.
		message = ts.flattenDiagnosticMessageText(diagnostic.messageText, "\n");
	}

	return {
		location,
		severity,
		message,
	};
};

let findRenameLocations = (request) => {
	// Get the source file and position.
	let sourceFile = host.getSourceFile(request.url);
	let position = ts.getPositionOfLineAndCharacter(
		sourceFile,
		request.position.line,
		request.position.character,
	);

	// Get the rename locations.
	let renameLocations = languageService.findRenameLocations(
		request.url,
		position,
		false,
		false,
	);

	// Convert the definitions.
	let locations = renameLocations?.map((renameLocation) => {
		let destFile = host.getSourceFile(renameLocation.fileName);
		// Get the definitions's range.
		let start = ts.getLineAndCharacterOfPosition(
			destFile,
			renameLocation.textSpan.start,
		);
		let end = ts.getLineAndCharacterOfPosition(
			destFile,
			renameLocation.textSpan.start + renameLocation.textSpan.length,
		);
		let location = {
			url: renameLocation.fileName,
			range: { start, end },
		};
		return location;
	});

	return {
		type: "find_rename_locations",
		response: { locations },
	};
};

let hover = (request) => {
	// Get the source file and position.
	let sourceFile = host.getSourceFile(request.url);
	let position = ts.getPositionOfLineAndCharacter(
		sourceFile,
		request.position.line,
		request.position.character,
	);

	// Get the quick info at the position.
	let quickInfo = languageService.getQuickInfoAtPosition(request.url, position);

	// Get the text.
	let text = quickInfo?.displayParts?.map(({ text }) => text).join("");

	return {
		type: "get_hover",
		response: { text },
	};
};

let gotoDefinition = (request) => {
	// Get the source file and position.
	let sourceFile = host.getSourceFile(request.url);
	let position = ts.getPositionOfLineAndCharacter(
		sourceFile,
		request.position.line,
		request.position.character,
	);

	// Get the definitions.
	let definitions = languageService.getDefinitionAtPosition(
		request.url,
		position,
	);

	// Convert the definitions.
	let locations = definitions?.map((definition) => {
		let destFile = host.getSourceFile(definition.fileName);
		// Get the definitions's range.
		let start = ts.getLineAndCharacterOfPosition(
			destFile,
			definition.textSpan.start,
		);
		let end = ts.getLineAndCharacterOfPosition(
			destFile,
			definition.textSpan.start + definition.textSpan.length,
		);

		let location = {
			url: definition.fileName,
			range: { start, end },
		};

		return location;
	});

	return {
		type: "goto_definition",
		response: { locations },
	};
};

let completion = (request) => {
	// Get the source file and position.
	let sourceFile = host.getSourceFile(request.url);
	let position = ts.getPositionOfLineAndCharacter(
		sourceFile,
		request.position.line,
		request.position.character,
	);

	// Get the completions.
	let info = languageService.getCompletionsAtPosition(request.url, position);

	// Convert the completion entries.
	let entries = info?.entries.map((entry) => ({ name: entry.name }));

	return {
		type: "completion",
		response: { entries },
	};
};

let getReferences = (request) => {
	// Get the source file and position.
	let sourceFile = host.getSourceFile(request.url);
	let position = ts.getPositionOfLineAndCharacter(
		sourceFile,
		request.position.line,
		request.position.character,
	);
	let references = languageService.getReferencesAtPosition(
		request.url,
		position,
	);

	// Convert the references.
	let locations = references?.map((reference) => {
		let destFile = host.getSourceFile(reference.fileName);
		// Get the references's range.
		let start = ts.getLineAndCharacterOfPosition(
			destFile,
			reference.textSpan.start,
		);
		let end = ts.getLineAndCharacterOfPosition(
			destFile,
			reference.textSpan.start + reference.textSpan.length,
		);

		let location = {
			url: reference.fileName,
			range: { start, end },
		};

		return location;
	});

	return {
		type: "get_references",
		response: { locations },
	};
};

let transpile = (request) => {
	// Transpile.
	let output = ts.transpileModule(request.source, {
		compilerOptions: {
			module: ts.ModuleKind.ESNext,
			target: ts.ScriptTarget.ESNext,
			sourceMap: true,
		},
	});

	return {
		type: "transpile",
		response: {
			outputText: output.outputText,
			sourceMapText: output.sourceMapText,
		},
	};
};

globalThis.console = {
	log: (...args) => {
		let string = args.map((arg) => stringify(arg)).join(" ");
		syscall(Syscall.Print, string);
	},
	error: (...args) => {
		let string = args.map((arg) => stringify(arg)).join(" ");
		syscall(Syscall.Print, string);
	},
};

let stringify = (value) => {
	let inner = (value, visited) => {
		let type = typeof value;
		switch (type) {
			case "string": {
				return `"${value}"`;
			}
			case "number": {
				return value.toString();
			}
			case "boolean": {
				return value ? "true" : "false";
			}
			case "undefined": {
				return "undefined";
			}
			case "object": {
				if (value === null) {
					return "null";
				}
				if (visited.has(value)) {
					return "[circular]";
				}
				visited.add(value);
				if (Array.isArray(value)) {
					return `[${value.map((value) => inner(value, visited)).join(", ")}]`;
				} else if (value instanceof Error) {
					return value.stack;
				} else if (value instanceof Promise) {
					return "[promise]";
				} else {
					let constructorName = "";
					if (
						value.constructor !== undefined &&
						value.constructor.name !== "Object"
					) {
						constructorName = `${value.constructor.name} `;
					}
					let entries = Object.entries(value).map(
						([key, value]) => `${key}: ${inner(value, visited)}`,
					);
					return `${constructorName}{ ${entries.join(", ")} }`;
				}
			}
			case "function": {
				return `[function ${value.name ?? "(anonymous)"}]`;
			}
			case "symbol": {
				return "[symbol]";
			}
			case "bigint": {
				return value.toString();
			}
		}
	};
	return inner(value, new Set());
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
			return globalThis.Deno.core.opSync(opName, ...args);
		case Syscall.OpenedFiles:
			return globalThis.Deno.core.opSync(opName, ...args);
		case Syscall.Print:
			return globalThis.Deno.core.opSync(opName, ...args);
		case Syscall.Resolve:
			return globalThis.Deno.core.opSync(opName, ...args);
		case Syscall.Version:
			return globalThis.Deno.core.opSync(opName, ...args);
	}
};
