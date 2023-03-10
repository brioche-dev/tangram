import { Location } from "./location";
import { languageService } from "./typescript";
import * as ts from "typescript";

export type Request = {};

export type Response = {
	diagnostics: Record<string, Array<Diagnostic>>;
};
export type Diagnostic = {
	location: Location | null;
	severity: Severity;
	message: string;
};

export type Severity = "error" | "warning" | "information" | "hint";

export let handle = (_request: Request): Response => {
	// Get the module identifiers of all documents.
	let moduleIdentifiers = syscall("get_documents");

	// Collect the diagnostics.
	let diagnostics: Record<string, Array<Diagnostic>> = {};
	for (let moduleIdentifier of moduleIdentifiers) {
		diagnostics[moduleIdentifier] = [
			...languageService.getSyntacticDiagnostics(moduleIdentifier),
			...languageService.getSemanticDiagnostics(moduleIdentifier),
			...languageService.getSuggestionDiagnostics(moduleIdentifier),
		].map((diagnostic) => convertDiagnosticFromTypeScript(diagnostic));
	}

	return {
		diagnostics,
	};
};

/** Convert diagnostics from TypeScript. */
export let convertDiagnosticsFromTypeScript = (
	diagnostics: Array<ts.Diagnostic>,
): Record<string, Array<Diagnostic>> => {
	let output: Record<string, Array<Diagnostic>> = {};

	for (let diagnostic of diagnostics) {
		// Ignore diagnostics that do not have a file.
		if (diagnostic.file === undefined) {
			continue;
		}

		// Get the module identifier.
		let moduleIdentifier = diagnostic.file.fileName;

		// Add an entry for this diagnostic's module identifier in the output if necessary.
		if (output[moduleIdentifier] === undefined) {
			output[moduleIdentifier] = [];
		}

		// Add the diagnostic to the output.
		output[moduleIdentifier]?.push(convertDiagnosticFromTypeScript(diagnostic));
	}

	return output;
};

/** Convert a diagnostic from TypeScript. */
export let convertDiagnosticFromTypeScript = (
	diagnostic: ts.Diagnostic,
): Diagnostic => {
	if (
		diagnostic.file == undefined ||
		diagnostic.start === undefined ||
		diagnostic.length === undefined
	) {
		throw new Error("The diagnostic does not have a location.");
	}

	// Get the diagnostic's module identifier.
	let moduleIdentifier = diagnostic.file.fileName;

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
			moduleIdentifier,
			range,
		};
	}

	// Convert the diagnostic's severity.
	let severity: Severity;
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

	let message: string;
	// Map diagnostics for '.ts' extensions to import errors instead.
	if (diagnostic.code === 2691) {
		// TS2691: An import path cannot end with a '.ts' extension. Consider importing 'bad-module' instead.
		message = "Could not load the module.";
	} else if (diagnostic.code === 2792) {
		// TS2792: Cannot find module. Did you mean to set the 'moduleResolution' option to 'node', or to add aliases to the 'paths' option?
		message = "Could not load the module.";
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
