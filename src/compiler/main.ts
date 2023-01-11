import { Request, Response } from "./request";
import { completion } from "./completion";
import { check } from "./check";
import { getDiagnostics } from "./diagnostics";
import { getReferences } from "./references";
import { gotoDefinition } from "./definition";
import { transpile } from "./transpile";
import { rename } from "./rename";
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
		case "rename": {
			let response = rename(request);
			return { type: "rename", response };
		}
		case "format": {
			let response = format(request);
			return { type: "format", response };
		}
		case "diagnostics": {
			let response = getDiagnostics(request);
			return { type: "diagnostics", response };
		}
		case "hover": {
			let response = hover(request);
			return { type: "hover", response };
		}
		case "references": {
			let response = getReferences(request);
			return { type: "references", response };
		}
		case "definition": {
			let response = gotoDefinition(request);
			return { type: "definition", response };
		}
		case "transpile": {
			let response = transpile(request);
			return { type: "transpile", response };
		}
	}
};
