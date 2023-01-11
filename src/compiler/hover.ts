import * as ts from "typescript";
import { HoverRequest, HoverResponse } from "./request";
import { host, languageService } from "./typescript";

export let hover = (request: HoverRequest): HoverResponse => {
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

	// Get the quick info at the position.
	let quickInfo = languageService.getQuickInfoAtPosition(
		request.moduleIdentifier,
		position,
	);

	// Get the text.
	let text = quickInfo?.displayParts?.map(({ text }) => text).join("");

	return { text };
};
