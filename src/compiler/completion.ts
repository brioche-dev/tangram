import * as ts from "typescript";
import { host, languageService } from "./typescript";
import { CompletionRequest, CompletionResponse } from "./request";

export let completion = (request: CompletionRequest): CompletionResponse => {
	// Get the source file and position.
	let sourceFile = host.getSourceFile(
		request.moduleIdentifier,
		ts.ScriptTarget.ESNext,
	);
	if (sourceFile === undefined) {
		throw new Error();
	}
	let position = ts.getPositionOfLineAndCharacter(
		sourceFile,
		request.position.line,
		request.position.character,
	);

	// Get the completions.
	let info = languageService.getCompletionsAtPosition(
		request.moduleIdentifier,
		position,
		undefined,
	);

	// Convert the completion entries.
	let entries = info?.entries.map((entry) => ({ name: entry.name }));

	return {
		entries,
	};
};
