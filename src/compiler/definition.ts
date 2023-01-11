import * as ts from "typescript";
import { Location, Position } from "./types";
import { host, languageService } from "./typescript";

export type DefinitionRequest = {
	moduleIdentifier: string;
	position: Position;
};

export type DefinitionResponse = {
	locations: Array<Location> | null;
};

export let definition = (request: DefinitionRequest): DefinitionResponse => {
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

	// Get the definitions.
	let definitions = languageService.getDefinitionAtPosition(
		request.moduleIdentifier,
		position,
	);

	// Convert the definitions.
	let locations =
		definitions?.map((definition) => {
			let destFile = host.getSourceFile(
				definition.fileName,
				ts.ScriptTarget.ESNext,
			);
			if (destFile === undefined) {
				throw new Error();
			}
			// Get the definitions's range.
			let start = ts.getLineAndCharacterOfPosition(
				destFile,
				definition.textSpan.start,
			);
			let end = ts.getLineAndCharacterOfPosition(
				destFile,
				definition.textSpan.start + definition.textSpan.length,
			);

			let location = {
				moduleIdentifier: definition.fileName,
				range: { start, end },
			};

			return location;
		}) ?? null;

	return {
		locations,
	};
};
