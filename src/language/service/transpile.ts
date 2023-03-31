import ts from "typescript";

export type Request = {
	text: string;
};

export type Response = {
	transpiledText: string;
	sourceMapText: string;
};

export let handle = (request: Request): Response => {
	// Transpile.
	let output = ts.transpileModule(request.text, {
		compilerOptions: {
			module: ts.ModuleKind.ESNext,
			target: ts.ScriptTarget.ESNext,
			sourceMap: true,
		},
	});

	// Retrieve the output and source map.
	let transpiledText = output.outputText;
	let sourceMapText = output.sourceMapText;
	if (sourceMapText === undefined) {
		throw new Error("Expected source map.");
	}

	return {
		transpiledText,
		sourceMapText,
	};
};
