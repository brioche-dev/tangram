import * as ts from "typescript";
import { convertDiagnostics } from "./diagnostics";
import { host, compilerOptions } from "./typescript";
import { CheckRequest, CheckResponse } from "./types";

export let check = (request: CheckRequest): CheckResponse => {
	// Create a typescript program.
	let program = ts.createProgram({
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
		diagnostics,
	};
};
