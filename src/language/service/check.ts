import { Diagnostic, convertDiagnosticsFromTypeScript } from "./diagnostics";
import { compilerOptions, host } from "./typescript";
import * as ts from "typescript";

export type Request = {
	moduleIdentifiers: Array<string>;
};

export type Response = {
	diagnostics: Record<string, Array<Diagnostic>>;
};

export let handle = (request: Request): Response => {
	// Create a typescript program.
	let program = ts.createProgram({
		rootNames: request.moduleIdentifiers,
		options: compilerOptions,
		host,
	});

	// Get the diagnostics and convert them.
	let diagnostics = convertDiagnosticsFromTypeScript([
		...program.getConfigFileParsingDiagnostics(),
		...program.getOptionsDiagnostics(),
		...program.getGlobalDiagnostics(),
		...program.getDeclarationDiagnostics(),
		...program.getSyntacticDiagnostics(),
		...program.getSemanticDiagnostics(),
	]);

	return {
		diagnostics,
	};
};
