import { Blob, blob } from "./blob";
import { Dependency, dependency } from "./dependency";
import { Directory, directory } from "./directory";
import { download } from "./download";
import { file, File } from "./file";
import { currentPackage, Package } from "./package";
import { Path, path } from "./path";
import { placeholder, Placeholder } from "./placeholder";
import { process, output } from "./process";
import { resolve } from "./resolve";
import { symlink, Symlink } from "./symlink";
import { target, createTarget } from "./target";
import { t, Template, template } from "./template";

let tg = {
	Blob,
	blob,
	Dependency,
	dependency,
	Directory,
	directory,
	download,
	file,
	File,
	currentPackage,
	Package,
	Path,
	path,
	placeholder,
	Placeholder,
	process,
	output,
	resolve,
	symlink,
	Symlink,
	target,
	createTarget,
	Template,
	template,
};

let stringify = (value: unknown): string => {
	let inner = (value: unknown, visited: Set<unknown>): string => {
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
				}
				if (visited.has(value)) {
					return "[circular]";
				}
				visited.add(value);

				if (Array.isArray(value)) {
					return `[${value.map((value) => inner(value, visited)).join(", ")}]`;
				} else if (value instanceof Error) {
					return value.stack ?? "";
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

let console = {
	log: (...args: Array<unknown>) => {
		let string = args.map((arg) => stringify(arg)).join(" ");
		syscall("print", string);
	},
};

Object.defineProperties(globalThis, {
	console: { value: console },
	tg: { value: tg },
	t: { value: t },
});
