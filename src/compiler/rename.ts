import * as ts from "typescript";
import { Location, Position } from "./types";
import { host, languageService } from "./typescript";

export type RenameRequest = {
	moduleIdentifier: string;
	position: Position;
};

export type RenameResponse = {
	locations: Array<Location> | null | undefined;
};

export let rename = (request: RenameRequest): RenameResponse => {
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

	// Get the rename locations.
	let renameLocations = languageService.findRenameLocations(
		request.moduleIdentifier,
		position,
		false,
		false,
	);

	// Convert the definitions.
	let locations = renameLocations?.map((renameLocation) => {
		let destFile = host.getSourceFile(
			renameLocation.fileName,
			ts.ScriptTarget.ESNext,
		);
		if (destFile === undefined) {
			throw new Error();
		}
		// Get the definitions's range.
		let start = ts.getLineAndCharacterOfPosition(
			destFile,
			renameLocation.textSpan.start,
		);
		let end = ts.getLineAndCharacterOfPosition(
			destFile,
			renameLocation.textSpan.start + renameLocation.textSpan.length,
		);
		let location = {
			moduleIdentifier: renameLocation.fileName,
			range: { start, end },
		};
		return location;
	});

	return { locations };
};
