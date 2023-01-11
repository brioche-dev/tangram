import * as ts from "typescript";
import { GetReferencesRequest, GetReferencesResponse } from "./request";
import { host, languageService } from "./typescript";

export let getReferences = (
	request: GetReferencesRequest,
): GetReferencesResponse => {
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
	let references = languageService.getReferencesAtPosition(
		request.moduleIdentifier,
		position,
	);

	// Convert the references.
	let locations =
		references?.map((reference) => {
			let destFile = host.getSourceFile(
				reference.fileName,
				ts.ScriptTarget.ESNext,
			);
			if (destFile === undefined) {
				throw new Error(destFile);
			}
			// Get the references's range.
			let start = ts.getLineAndCharacterOfPosition(
				destFile,
				reference.textSpan.start,
			);
			let end = ts.getLineAndCharacterOfPosition(
				destFile,
				reference.textSpan.start + reference.textSpan.length,
			);

			let location = {
				url: reference.fileName,
				range: { start, end },
			};

			return location;
		}) ?? null;

	return {
		locations,
	};
};
