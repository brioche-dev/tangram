import { Location } from "./location.ts";
import { Position } from "./position.ts";
import { ModuleIdentifier } from "./syscall.ts";
import * as typescript from "./typescript.ts";
import { nullish } from "./util.ts";
import ts from "typescript";

export type Request = {
	moduleIdentifier: ModuleIdentifier;
	position: Position;
};

export type Response = {
	locations: Array<Location> | nullish;
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

	// Get the rename locations.
	let renameLocations = typescript.languageService.findRenameLocations(
		typescript.fileNameFromModuleIdentifier(request.moduleIdentifier),
		position,
		false,
		false,
	);

	// Convert the definitions.
	let locations = renameLocations?.map((renameLocation) => {
		let destFile = typescript.host.getSourceFile(
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
			moduleIdentifier: typescript.moduleIdentifierFromFileName(
				renameLocation.fileName,
			),
			range: { start, end },
		};
		return location;
	});

	return { locations };
};
