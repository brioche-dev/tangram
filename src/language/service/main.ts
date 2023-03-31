import * as check from "./check.ts";
import * as completion from "./completion.ts";
import * as definition from "./definition.ts";
import * as diagnostics from "./diagnostics.ts";
import * as doc from "./doc.ts";
import { prepareStackTrace } from "./error.ts";
import * as format from "./format.ts";
import * as hover from "./hover.ts";
import * as imports from "./imports.ts";
import * as metadata from "./metadata.ts";
import * as references from "./references.ts";
import * as rename from "./rename.ts";
import * as transpile from "./transpile.ts";

// Set `Error.prepareStackTrace`.
Object.defineProperties(Error, {
	prepareStackTrace: { value: prepareStackTrace },
});

type Request =
	| { kind: "check"; request: check.Request }
	| { kind: "completion"; request: completion.Request }
	| { kind: "definition"; request: definition.Request }
	| { kind: "diagnostics"; request: diagnostics.Request }
	| { kind: "doc"; request: doc.Request }
	| { kind: "format"; request: format.Request }
	| { kind: "hover"; request: hover.Request }
	| { kind: "imports"; request: imports.Request }
	| { kind: "metadata"; request: metadata.Request }
	| { kind: "references"; request: references.Request }
	| { kind: "rename"; request: rename.Request }
	| { kind: "transpile"; request: transpile.Request };

type Response =
	| { kind: "check"; response: check.Response }
	| { kind: "completion"; response: completion.Response }
	| { kind: "definition"; response: definition.Response }
	| { kind: "diagnostics"; response: diagnostics.Response }
	| { kind: "doc"; response: doc.Response }
	| { kind: "format"; response: format.Response }
	| { kind: "hover"; response: hover.Response }
	| { kind: "imports"; response: imports.Response }
	| { kind: "metadata"; response: metadata.Response }
	| { kind: "references"; response: references.Response }
	| { kind: "rename"; response: rename.Response }
	| { kind: "transpile"; response: transpile.Response };

let handle = ({ kind, request }: Request): Response => {
	switch (kind) {
		case "check": {
			let response = check.handle(request);
			return { kind: "check", response };
		}
		case "completion": {
			let response = completion.handle(request);
			return { kind: "completion", response };
		}
		case "definition": {
			let response = definition.handle(request);
			return { kind: "definition", response };
		}
		case "diagnostics": {
			let response = diagnostics.handle(request);
			return { kind: "diagnostics", response };
		}
		case "doc": {
			let response = doc.handle(request);
			return { kind: "doc", response };
		}
		case "format": {
			let response = format.handle(request);
			return { kind: "format", response };
		}
		case "hover": {
			let response = hover.handle(request);
			return { kind: "hover", response };
		}
		case "imports": {
			let response = imports.handle(request);
			return { kind: "imports", response };
		}
		case "metadata": {
			let response = metadata.handle(request);
			return { kind: "metadata", response };
		}
		case "references": {
			let response = references.handle(request);
			return { kind: "references", response };
		}
		case "rename": {
			let response = rename.handle(request);
			return { kind: "rename", response };
		}
		case "transpile": {
			let response = transpile.handle(request);
			return { kind: "transpile", response };
		}
	}
};

Object.defineProperties(globalThis, {
	handle: { value: handle },
});
