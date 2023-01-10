import * as ts from "typescript";
import {
	FindRenameLocationsRequest,
	FindRenameLocationsResponse,
} from "./request";
import { host, languageService } from "./typescript";

export let findRenameLocations = (
	request: FindRenameLocationsRequest,
): FindRenameLocationsResponse => {
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

	// Get the rename locations.
	let renameLocations = languageService.findRenameLocations(
		request.url,
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
			url: renameLocation.fileName,
			range: { start, end },
		};
		return location;
	});

	return {
		type: "find_rename_locations",
		response: { locations },
	};
};
