import { unreachable } from "./assert.ts";
import { Blob } from "./blob.ts";
import { Directory } from "./directory.ts";
import * as encoding from "./encoding.ts";
import { File } from "./file.ts";
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
	} else if (Blob.is(value)) {
		let handle = stringifyHandle(value.handle, visited);
		return `(tg.blob ${handle})`;
	} else if (Directory.is(value)) {
		let handle = stringifyHandle(value.handle, visited);
		return `(tg.directory ${handle})`;
	} else if (File.is(value)) {
		let handle = stringifyHandle(value.handle, visited);
		return `(tg.file ${handle})`;
	} else if (Symlink.is(value)) {
		let handle = stringifyHandle(value.handle, visited);
		return `(tg.symlink ${handle})`;
	} else if (Template.is(value)) {
		let string = value.components
			.map((component) => {
				if (typeof component === "string") {
					return component;
				} else {
					return `\${${stringifyInner(component, visited)}}`;
				}
			})
			.join("");
		return `\`${string}\``;
	} else if (Mutation.is(value)) {
		return `(tg.mutation ${stringifyObject(value.inner, visited)})`;
	} else if (Package.is(value)) {
		let handle = stringifyHandle(value.handle, visited);
		return `(tg.package ${handle})`;
	} else if (Target.is(value)) {
		let handle = stringifyHandle(value.handle, visited);
		return `(tg.target ${handle})`;
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

let stringifyHandle = (
	handle: Object_.Handle,
	visited: WeakSet<object>,
): string => {
	let { id, object } = handle.state;
	if (id !== undefined) {
		return id;
	}
	if (object !== undefined) {
		return stringifyObject(object.value, visited);
	}
	return unreachable();
};
