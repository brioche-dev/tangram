let bs = globalThis.__bootstrap;
Object.defineProperties(globalThis, {
	TextDecoder: { value: bs.encoding.TextDecoder },
	TextEncoder: { value: bs.encoding.TextEncoder },
	URL: { value: bs.url.URL },
	URLPattern: { value: bs.urlPattern.URLPattern },
	URLSearchParams: { value: bs.url.URLSearchParams },
});

let stringify = (value) => {
	let inner = (value, visited) => {
		let type = typeof value;
		switch (type) {
			case "string": {
				return `"${value}"`;
			}
			case "number": {
				return value.toString();
			}
			case "boolean": {
				return value ? "true" : "false";
			}
			case "undefined": {
				return "undefined";
			}
			case "object": {
				if (value === null) {
					return "null";
				}
				if (visited.has(value)) {
					return "[circular]";
				}
				visited.add(value);
				if (Array.isArray(value)) {
					return `[${value.map((value) => inner(value, visited)).join(", ")}]`;
				} else if (value instanceof Error) {
					return value.stack;
				} else if (value instanceof Promise) {
					return "[promise]";
				} else {
					let constructorName = "";
					if (value.constructor?.name !== "Object") {
						constructorName = `${value.constructor?.name} `;
					}
					let entries = Object.entries(value).map(
						([key, value]) => `${key}: ${inner(value, visited)}`,
					);
					return `${constructorName}{ ${entries.join(", ")} }`;
				}
			}
			case "function": {
				return `[function ${value.name ?? "(anonymous)"}]`;
			}
			case "symbol": {
				return "[symbol]";
			}
		}
	};
	return inner(value, new Set());
};

Object.defineProperties(globalThis, {
	stringify: { value: stringify },
});

let console = {
	log: (...args) => {
		let string = args.map((arg) => stringify(arg)).join(" ");
		syscall("print", string);
	},
	error: (...args) => {
		let string = args.map((arg) => stringify(arg)).join(" ");
		syscall("print", string);
	},
};

Object.defineProperties(globalThis, {
	console: { value: console },
});

let Syscall = {
	Print: "print",
	Serialize: "serialize",
	Deserialize: "deserialize",
	AddBlob: "add_blob",
	GetBlob: "get_blob",
	AddExpression: "add_expression",
	GetExpression: "get_expression",
	Evaluate: "evaluate",
};

function syscall(syscall, ...args) {
	let opName = "op_tg_" + syscall;
	switch (syscall) {
		case "print":
			return Deno.core.opSync(opName, ...args);
		case "deserialize":
			return Deno.core.opSync(opName, ...args);
		case "add_blob":
			return Deno.core.opAsync(opName, ...args);
		case "get_blob":
			return Deno.core.opAsync(opName, ...args);
		case "add_expression":
			return Deno.core.opAsync(opName, ...args);
		case "get_expression":
			return Deno.core.opAsync(opName, ...args);
		case "evaluate":
			return Deno.core.opAsync(opName, ...args);
	}
}

let ExpressionType = {
	Null: "null",
	Bool: "bool",
	Number: "number",
	String: "string",
	Directory: "directory",
	File: "file",
	Symlink: "symlink",
	Dependency: "dependency",
	Template: "template",
	Package: "package",
	Js: "js",
	Fetch: "fetch",
	Process: "process",
	Target: "target",
	Array: "array",
	Map: "map",
};

let typeSymbol = Symbol();

let Tangram = {
	Syscall,
	syscall,
	ExpressionType,
	typeSymbol,
};

Object.defineProperties(globalThis, {
	Tangram: { value: Tangram },
});
