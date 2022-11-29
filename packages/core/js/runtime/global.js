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
					if (
						value.constructor !== undefined &&
						value.constructor.name !== "Object"
					) {
						constructorName = `${value.constructor.name} `;
					}

					// If the value has a `.toString()` method, which returns a string, use that as the body of the debug output.
					if ("toString" in value) {
						let result = value.toString();
						if (typeof result === "string") {
							return `${constructorName}{ ${result} }`;
						}
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
			case "bigint": {
				return value.toString();
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

function syscall(syscall, ...args) {
	let opName = "op_tg_" + syscall;
	switch (syscall) {
		case "get_hash":
			return Deno.core.opSync(opName, ...args);
		case "get_name":
			return Deno.core.opSync(opName, ...args);
		case "get_args":
			return Deno.core.opSync(opName, ...args);
		case "return":
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
		case "print":
			return Deno.core.opSync(opName, ...args);
		case "serialize":
			return Deno.core.opSync(opName, ...args);
		case "deserialize":
			return Deno.core.opSync(opName, ...args);
		default:
			throw new Error("Unknown syscall.");
	}
}

let typeSymbol = Symbol();

let Tangram = {
	syscall,
	typeSymbol,
};

Object.defineProperties(globalThis, {
	Tangram: { value: Tangram },
});
