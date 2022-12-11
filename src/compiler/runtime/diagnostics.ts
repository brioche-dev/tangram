import * as ts from "typescript";
import {
	Diagnostic,
	GetDiagnosticsResponse,
	GetDiangosticsRequest,
	Severity,
} from "./types";
import { languageService } from "./typescript";

export let getDiagnostics = (
	_request: GetDiangosticsRequest,
): GetDiagnosticsResponse => {
	// Get the list of opened files.
	let urls = syscall("opened_files");

	// Collect the diagnostics for each opened file.
	let diagnostics: Record<string, Array<Diagnostic>> = {};
	for (let url of urls) {
		diagnostics[url] = [
			...languageService.getSyntacticDiagnostics(url),
			...languageService.getSemanticDiagnostics(url),
			...languageService.getSuggestionDiagnostics(url),
		].map((diagnostic) => convertDiagnostic(diagnostic));
	}

	return {
		diagnostics,
	};
};

/** Convert TypeScript diagnostics to Tangram diagnostics. */
export let convertDiagnostics = (
	diagnostics: Array<ts.Diagnostic>,
): Record<string, Array<Diagnostic>> => {
	let output: Record<string, Array<Diagnostic>> = {};

	for (let diagnostic of diagnostics) {
		// Ignore diagnostics that do not have a file.
		if (diagnostic.file === undefined) {
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
export let convertDiagnostic = (diagnostic: ts.Diagnostic): Diagnostic => {
	if (
		diagnostic.file == undefined ||
		diagnostic.start === undefined ||
		diagnostic.length === undefined
	) {
		throw new Error("The diagnostic does not have a location.");
	}

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
	// Map diagnostics for '.ts' extensions to url import errors instead.
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
