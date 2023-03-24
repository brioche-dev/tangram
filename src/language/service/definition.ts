import { Location } from "./location";
import { Position } from "./position";
import { ModuleIdentifier } from "./syscall";
import * as typescript from "./typescript";
import { nullish } from "./util";
import * as ts from "typescript";

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

	// Get the definitions.
	let definitions = typescript.languageService.getDefinitionAtPosition(
		typescript.fileNameFromModuleIdentifier(request.moduleIdentifier),
		position,
	);

	// Convert the definitions.
	let locations = definitions?.map((definition) => {
		let destFile = typescript.host.getSourceFile(
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
			moduleIdentifier: typescript.moduleIdentifierFromFileName(
				definition.fileName,
			),
			range: { start, end },
		};

		return location;
	});

	return {
		locations,
	};
};
