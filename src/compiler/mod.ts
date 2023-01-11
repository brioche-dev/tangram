import {
	completion,
	CompletionRequest,
	CompletionResponse,
} from "./completion";
import { check, CheckRequest, CheckResponse } from "./check";
import {
	DiagnosticsResponse,
	DiangosticsRequest,
	getDiagnostics,
} from "./diagnostics";
import {
	getReferences,
	ReferencesRequest,
	ReferencesResponse,
} from "./references";
import {
	definition,
	DefinitionRequest,
	DefinitionResponse,
} from "./definition";
import { transpile, TranspileRequest, TranspileResponse } from "./transpile";
import { rename, RenameRequest, RenameResponse } from "./rename";
import { hover, HoverRequest, HoverResponse } from "./hover";
import { format, FormatRequest, FormatResponse } from "./format";

type Request =
	| { type: "check"; request: CheckRequest }
	| { type: "completion"; request: CompletionRequest }
	| { type: "rename"; request: RenameRequest }
	| { type: "format"; request: FormatRequest }
	| { type: "diagnostics"; request: DiangosticsRequest }
	| { type: "hover"; request: HoverRequest }
	| { type: "references"; request: ReferencesRequest }
	| { type: "definition"; request: DefinitionRequest }
	| { type: "transpile"; request: TranspileRequest };

type Response =
	| { type: "check"; response: CheckResponse }
	| { type: "completion"; response: CompletionResponse }
	| { type: "rename"; response: RenameResponse }
	| { type: "format"; response: FormatResponse }
	| { type: "diagnostics"; response: DiagnosticsResponse }
	| { type: "hover"; response: HoverResponse }
	| { type: "references"; response: ReferencesResponse }
	| { type: "definition"; response: DefinitionResponse }
	| { type: "transpile"; response: TranspileResponse };

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
			let response = definition(request);
			return { type: "definition", response };
		}
		case "transpile": {
			let response = transpile(request);
			return { type: "transpile", response };
		}
	}
};
