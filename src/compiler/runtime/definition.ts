import * as ts from "typescript";
import { GotoDefinitionRequest, GotoDefinitionResponse } from "./types";
import { host, languageService } from "./typescript";

export let gotoDefinition = (
	request: GotoDefinitionRequest,
): GotoDefinitionResponse => {
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

	// Get the definitions.
	let definitions = languageService.getDefinitionAtPosition(
		request.url,
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
				url: definition.fileName,
				range: { start, end },
			};

			return location;
		}) ?? null;

	return {
		locations,
	};
};
