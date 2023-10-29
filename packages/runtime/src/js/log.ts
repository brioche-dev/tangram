import { unreachable } from "./assert.ts";
import { Branch } from "./branch.ts";
import { Directory } from "./directory.ts";
import * as encoding from "./encoding.ts";
import { File } from "./file.ts";
import { Leaf } from "./leaf.ts";
import { Mutation } from "./mutation.ts";
import { Object_ } from "./object.ts";
import { Package } from "./package.ts";
import { Symlink } from "./symlink.ts";
import * as syscall from "./syscall.ts";
import { Target } from "./target.ts";
import { Template } from "./template.ts";

export let log = (...args: Array<unknown>) => {
	let string = args.map((arg) => stringify(arg)).join(" ") + "\n";
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
			if (Target.is(value)) {
				return stringifyObject(value, visited);
			} else {
				return `(function "${value.name ?? "(anonymous)"}")`;
			}
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
	} else if (value instanceof Uint8Array) {
		let bytes = encoding.hex.encode(value);
		return `(tg.bytes ${bytes})`;
	} else if (value instanceof Error) {
		return value.message;
	} else if (value instanceof Promise) {
		return "(promise)";
	} else if (Leaf.is(value)) {
		return stringifyHandle(value.handle, visited);
	} else if (Branch.is(value)) {
		return stringifyHandle(value.handle, visited);
	} else if (Directory.is(value)) {
		return stringifyHandle(value.handle, visited);
	} else if (File.is(value)) {
		return stringifyHandle(value.handle, visited);
	} else if (Symlink.is(value)) {
		return stringifyHandle(value.handle, visited);
	} else if (Template.is(value)) {
		return `\`${value.components
			.map((component) => {
				if (typeof component === "string") {
					return component;
				} else {
					return `\${${stringifyInner(component, visited)}}`;
				}
			})
			.join("")}\``;
	} else if (Mutation.is(value)) {
		return `(tg.mutation ${stringifyObject(value.inner, visited)})`;
	} else if (Package.is(value)) {
		return stringifyHandle(value.handle, visited);
	} else if (Target.is(value)) {
		return stringifyHandle(value.handle, visited);
	} else {
		let string = "";
		if (
			value.constructor !== undefined &&
			value.constructor.name !== "Object"
		) {
			string += `${value.constructor.name} `;
		}
		string += "{";
		let entries = Object.entries(value);
		if (entries.length > 0) {
			string += " ";
		}
		string += entries
			.map(([key, value]) => `${key}: ${stringifyInner(value, visited)}`)
			.join(", ");
		if (entries.length > 0) {
			string += " ";
		}
		string += "}";
		return string;
	}
};

let stringifyHandle = (
	handle: Object_.Handle,
	visited: WeakSet<object>,
): string => {
	let { id, object } = handle.state;
	if (id !== undefined) {
		return id;
	} else if (object !== undefined) {
		return `(tg.${object.kind} ${stringifyObject(object.value, visited)})`;
	} else {
		return unreachable();
	}
};
