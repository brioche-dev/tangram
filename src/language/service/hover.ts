import { Position } from "./position.ts";
import { ModuleIdentifier } from "./syscall.ts";
import * as typescript from "./typescript.ts";
import ts from "typescript";

export type Request = {
	moduleIdentifier: ModuleIdentifier;
	position: Position;
};

export type Response = {
	text?: string;
};

export let handle = (request: Request): Response => {
	// Get the source file.
	let sourceFile = typescript.host.getSourceFile(
		typescript.fileNameFromModuleIdentifier(request.moduleIdentifier),
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
	let quickInfo = typescript.languageService.getQuickInfoAtPosition(
		typescript.fileNameFromModuleIdentifier(request.moduleIdentifier),
		position,
	);

	// Get the text.
	let text = quickInfo?.displayParts?.map(({ text }) => text).join("");

	return { text };
};
