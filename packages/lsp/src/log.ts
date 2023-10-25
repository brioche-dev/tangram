import * as syscall from "./syscall.ts";

export let log = (...args: Array<unknown>) => {
	let string = args.map((arg) => stringify(arg)).join(" ");
	syscall.log(string);
};

let stringify = (value: unknown): string => {
	return stringifyInner(value, new WeakSet());
};

let stringifyInner = (value: unknown, visited: WeakSet<object>): string => {
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
			if (value === null) {
				return "null";
			} else {
				return stringifyObject(value, visited);
			}
		}
		case "function": {
			return `(function "${value.name ?? "(anonymous)"}")`;
		}
		case "symbol": {
			return "(symbol)";
		}
		case "bigint": {
			return value.toString();
		}
	}
};

let stringifyObject = (value: object, visited: WeakSet<object>): string => {
	if (visited.has(value)) {
		return "(circular)";
	}
	visited.add(value);
	if (value instanceof Array) {
		return `[${value
			.map((value) => stringifyInner(value, visited))
			.join(", ")}]`;
	} else if (value instanceof Error) {
		return value.message;
	} else if (value instanceof Promise) {
		return "(promise)";
	} else {
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
		let space = entries.length > 0 ? " " : "";
		return `${constructorName}{${space}${entries.join(", ")}${space}}`;
	}
};
