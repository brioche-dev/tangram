import * as ts from "typescript";
import { GetHoverRequest, GetHoverResponse } from "./request";
import { host, languageService } from "./typescript";

export let hover = (request: GetHoverRequest): GetHoverResponse => {
	// Get the source file and position.
	let sourceFile = host.getSourceFile(request.url, ts.ScriptTarget.ESNext);
	if (sourceFile === undefined) {
		throw new Error();
	}
	let position = ts.getPositionOfLineAndCharacter(
		sourceFile,
		request.position.line,
		request.position.character,
	);

	// Get the quick info at the position.
	let quickInfo = languageService.getQuickInfoAtPosition(request.url, position);

	// Get the text.
	let text = quickInfo?.displayParts?.map(({ text }) => text).join("");

	return { text };
};
