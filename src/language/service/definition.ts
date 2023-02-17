import { Location } from "./location";
import { Position } from "./position";
import { host, languageService } from "./typescript";
import { nullish } from "./util";
import * as ts from "typescript";

export type Request = {
	moduleIdentifier: string;
	position: Position;
};

export type Response = {
	locations: Array<Location> | nullish;
};

export let handle = (request: Request): Response => {
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
	let locations = definitions?.map((definition) => {
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
	});

	return {
		locations,
	};
};
