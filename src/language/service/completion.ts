import { Position } from "./position.ts";
import { ModuleIdentifier } from "./syscall.ts";
import * as typescript from "./typescript.ts";
import ts from "typescript";

export type Request = {
	moduleIdentifier: ModuleIdentifier;
	position: Position;
};

export type Response = {
	entries?: Array<CompletionEntry>;
};

export type CompletionEntry = {
	name: string;
};

export let handle = (request: Request): Response => {
	// Get the source file and position.
	let sourceFile = typescript.host.getSourceFile(
		typescript.fileNameFromModuleIdentifier(request.moduleIdentifier),
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
	let info = typescript.languageService.getCompletionsAtPosition(
		typescript.fileNameFromModuleIdentifier(request.moduleIdentifier),
		position,
		undefined,
	);

	// Convert the completion entries.
	let entries = info?.entries.map((entry) => ({ name: entry.name }));

	return {
		entries,
	};
};
