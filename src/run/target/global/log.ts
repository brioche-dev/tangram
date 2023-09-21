import { Blob } from "./blob.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Relpath, Subpath } from "./path.ts";
import { Placeholder } from "./placeholder.ts";
import { Resource } from "./resource.ts";
import { Symlink } from "./symlink.ts";
import * as syscall from "./syscall.ts";
import { Target } from "./target.ts";
import { Task } from "./task.ts";
import { Template } from "./template.ts";

/** Write to the log. */
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
	// If the value is in the visited set, then indicate that this is a circular reference.
	if (visited.has(value)) {
		return "(circular)";
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
		return "(promise)";
	} else if (value instanceof Relpath) {
		return `(tg.relpath ${value.toString()})`;
	} else if (value instanceof Subpath) {
		return `(tg.subpath ${value.toString()})`;
	} else if (value instanceof Blob) {
		return `(tg.blob ${value.id()})`;
	} else if (value instanceof Directory) {
		return `(tg.directory ${value.id()})`;
	} else if (value instanceof File) {
		return `(tg.file ${value.id()})`;
	} else if (value instanceof Symlink) {
		return `(tg.symlink ${value.id()})`;
	} else if (value instanceof Placeholder) {
		return `(tg.placeholder "${value.name()}")`;
	} else if (value instanceof Template) {
		let string = value
			.components()
			.map((component) => {
				if (typeof component === "string") {
					return component;
				} else {
					return `\${${stringifyInner(component, visited)}}`;
				}
			})
			.join("");
		return `(tg.template "${string}")`;
	} else if (value instanceof Resource) {
		return `(tg.resource "${value.id()}")`;
	} else if (value instanceof Target) {
		return `(tg.target "${value.id()}")`;
	} else if (value instanceof Task) {
		return `(tg.task "${value.id()}")`;
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
