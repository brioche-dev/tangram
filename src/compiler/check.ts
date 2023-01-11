import * as ts from "typescript";
import { convertDiagnostics } from "./diagnostics";
import { Diagnostic } from "./types";
import { host, compilerOptions } from "./typescript";

export type CheckRequest = { moduleIdentifiers: Array<string> };

export type CheckResponse = {
	diagnostics: { [key: string]: Array<Diagnostic> };
};

export let check = (request: CheckRequest): CheckResponse => {
	// Create a typescript program.
	let program = ts.createProgram({
		rootNames: [...request.moduleIdentifiers],
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
		diagnostics,
	};
};
