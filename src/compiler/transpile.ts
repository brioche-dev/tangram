import * as ts from "typescript";

export type TranspileRequest = {
	text: string;
};

export type TranspileResponse = {
	outputText: string;
	sourceMapText: string;
};

export let transpile = (request: TranspileRequest): TranspileResponse => {
	// Transpile.
	let output = ts.transpileModule(request.text, {
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
