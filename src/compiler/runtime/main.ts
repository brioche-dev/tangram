import { Request, Response } from "./types";
import { completion } from "./completion";
import { check } from "./check";
import { getDiagnostics } from "./diagnostics";
import { getReferences } from "./references";
import { gotoDefinition } from "./definition";
import { transpile } from "./transpile";
import { findRenameLocations } from "./rename";
import { hover } from "./hover";
import { format } from "./format";

export default ({ type, request }: Request): Response => {
	switch (type) {
		case "check": {
			let response = check(request);
			return { type: "check", response };
		}
		case "completion": {
			let response = completion(request);
			return { type: "completion", response };
		}
		case "find_rename_locations": {
			let response = findRenameLocations(request);
			return { type: "find_rename_locations", response };
		}
		case "format": {
			let response = format(request);
			return { type: "format", response };
		}
		case "get_diagnostics": {
			let response = getDiagnostics(request);
			return { type: "get_diagnostics", response };
		}
		case "get_hover": {
			let response = hover(request);
			return { type: "get_hover", response };
		}
		case "get_references": {
			let response = getReferences(request);
			return { type: "get_references", response };
		}
		case "goto_definition": {
			let response = gotoDefinition(request);
			return { type: "goto_definition", response };
		}
		case "transpile": {
			let response = transpile(request);
			return { type: "transpile", response };
		}
	}
};
