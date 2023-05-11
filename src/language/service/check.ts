import {
	Diagnostic,
	convertDiagnosticFromTypeScript,
	getLinterDiagnosticsForFile,
} from "./diagnostics.ts";
import { Module } from "./syscall.ts";
import * as typescript from "./typescript.ts";

import ts from "typescript";

export type Request = {
	modules: Array<Module>;
};

export type Response = {
	diagnostics: Array<Diagnostic>;
};

export let handle = (request: Request): Response => {
	// Create a typescript program.
	let program = ts.createProgram({
		rootNames: request.modules.map(typescript.fileNameFromModule),
		options: typescript.compilerOptions,
		host: typescript.host,
	});

	// Get the diagnostics and convert them.
	let diagnostics = [
		...program.getConfigFileParsingDiagnostics(),
		...program.getOptionsDiagnostics(),
		...program.getGlobalDiagnostics(),
		...program.getDeclarationDiagnostics(),
		...program.getSyntacticDiagnostics(),
		...program.getSemanticDiagnostics(),
	].map(convertDiagnosticFromTypeScript);

	let linterDiagnostics = program
		.getSourceFiles()
		.map((source) => getLinterDiagnosticsForFile(source))
		.flat();

	return {
		diagnostics: [...diagnostics, ...linterDiagnostics],
	};
};
