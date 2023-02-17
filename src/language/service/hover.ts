import { Position } from "./position";
import { host, languageService } from "./typescript";
import * as ts from "typescript";

export type Request = {
	moduleIdentifier: string;
	position: Position;
};

export type Response = {
	text?: string;
};

export let handle = (request: Request): Response => {
	// Get the source file.
	let sourceFile = host.getSourceFile(
		request.moduleIdentifier,
		ts.ScriptTarget.ESNext,
	);
	if (sourceFile === undefined) {
		throw new Error();
	}

	// Get the position of the hover.
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
