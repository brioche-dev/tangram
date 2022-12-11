import * as ts from "typescript";
import { TranspileRequest, TranspileResponse } from "./types";

export let transpile = (request: TranspileRequest): TranspileResponse => {
	// Transpile.
	let output = ts.transpileModule(request.source, {
		compilerOptions: {
			module: ts.ModuleKind.ESNext,
			target: ts.ScriptTarget.ESNext,
			sourceMap: true,
		},
	});

	let outputText = output.outputText;
	let sourceMapText = output.sourceMapText;
	if (sourceMapText === undefined) {
		throw new Error("Expected source map.");
	}

	return {
		outputText,
		sourceMapText,
	};
};
