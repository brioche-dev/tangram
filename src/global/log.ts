/** Write to the log. */
export let log = (...args: Array<unknown>) => {
	let string = args.map((arg) => stringify(arg)).join(" ");
	syscall("log", string);
};

let stringify = (value: unknown): string => {
	return stringifyInner(value, new Set());
};

let stringifyInner = (value: unknown, visited: Set<unknown>): string => {
	switch (typeof value) {
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
			return stringifyObject(value, visited);
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

let stringifyObject = (value: object | null, visited: Set<unknown>): string => {
	// Handle null.
	if (value === null) {
		return "null";
	}

	// If the value is in the visited set, then indicate that this is a circular reference.
	if (visited.has(value)) {
		return "[circular]";
	}

	// Add the value to the visited set.
	visited.add(value);

	if (value instanceof Array) {
		// Handle an array.
		return `[${value
			.map((value) => stringifyInner(value, visited))
			.join(", ")}]`;
	} else if (value instanceof Error) {
		// Handle an error.
		return value.stack ?? "";
	} else if (value instanceof Promise) {
		// Handle a promise.
		return "[promise]";
	} else {
		// Handle any other object.
		let constructorName = "";
		if (
			value.constructor !== undefined &&
			value.constructor.name !== "Object"
		) {
			constructorName = `${value.constructor.name} `;
		}
		let entries = Object.entries(value).map(
			([key, value]) => `${key}: ${stringifyInner(value, visited)}`,
		);
		return `${constructorName}{ ${entries.join(", ")} }`;
	}
};
