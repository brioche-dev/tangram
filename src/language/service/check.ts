import { Diagnostic, convertDiagnosticFromTypeScript } from "./diagnostics";
import { ModuleIdentifier } from "./syscall";
import * as typescript from "./typescript";
import * as ts from "typescript";

export type Request = {
	moduleIdentifiers: Array<ModuleIdentifier>;
};

export type Response = {
	diagnostics: Array<Diagnostic>;
};

export let handle = (request: Request): Response => {
	// Create a typescript program.
	let program = ts.createProgram({
		rootNames: request.moduleIdentifiers.map(
			typescript.fileNameFromModuleIdentifier,
		),
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

	throw new Error("This is an error in check.ts.");

	return {
		diagnostics,
	};
};
