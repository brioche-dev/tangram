import { Artifact } from "./artifact.ts";
import { assert as assert_, unreachable } from "./assert.ts";
import { Blob } from "./blob.ts";
import { Command } from "./command.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Operation } from "./operation.ts";
import { Relpath, Subpath } from "./path.ts";
import { Placeholder } from "./placeholder.ts";
import { Resource } from "./resource.ts";
import { Symlink } from "./symlink.ts";
import * as syscall from "./syscall.ts";
import { Template } from "./template.ts";

export type Value =
	| undefined
	| boolean
	| number
	| string
	| Uint8Array
	| Relpath
	| Subpath
	| Blob
	| Artifact
	| Placeholder
	| Template
	| Operation
	| Array<Value>
	| { [key: string]: Value };

export namespace Value {
	export let is = (value: unknown): value is Value => {
		return (
			value === undefined ||
			typeof value === "boolean" ||
			typeof value === "number" ||
			typeof value === "string" ||
			value instanceof Uint8Array ||
			value instanceof Relpath ||
			value instanceof Subpath ||
			value instanceof Blob ||
			value instanceof Directory ||
			value instanceof File ||
			value instanceof Symlink ||
			value instanceof Placeholder ||
			value instanceof Template ||
			value instanceof Command ||
			value instanceof Function ||
			value instanceof Resource ||
			value instanceof Array ||
			typeof value === "object"
		);
	};

	export let expect = (value: unknown): Value => {
		assert_(is(value));
		return value;
	};

	export let assert = (value: unknown): asserts value is Value => {
		assert_(is(value));
	};

	export let toSyscall = <T extends Value>(value: T): syscall.Value => {
		if (value === undefined) {
			return {
				kind: "null",
				value: null,
			};
		} else if (typeof value === "boolean") {
			return {
				kind: "bool",
				value,
			};
		} else if (typeof value === "number") {
			return {
				kind: "number",
				value,
			};
		} else if (typeof value === "string") {
			return {
				kind: "string",
				value,
			};
		} else if (value instanceof Uint8Array) {
			return {
				kind: "bytes",
				value,
			};
		} else if (value instanceof Relpath) {
			return {
				kind: "relpath",
				value: value.toSyscall(),
			};
		} else if (value instanceof Subpath) {
			return {
				kind: "subpath",
				value: value.toSyscall(),
			};
		} else if (value instanceof Blob) {
			return {
				kind: "blob",
				value: value.toSyscall(),
			};
		} else if (Artifact.is(value)) {
			return {
				kind: "artifact",
				value: Artifact.toSyscall(value),
			};
		} else if (value instanceof Placeholder) {
			return {
				kind: "placeholder",
				value: value.toSyscall(),
			};
		} else if (value instanceof Template) {
			return {
				kind: "template",
				value: value.toSyscall(),
			};
		} else if (Operation.is(value)) {
			return {
				kind: "operation",
				value: Operation.toSyscall(value),
			};
		} else if (value instanceof Array) {
			let syscallValue = value.map((value) => Value.toSyscall(value));
			return {
				kind: "array",
				value: syscallValue,
			};
		} else if (typeof value === "object") {
			let syscallValue = Object.fromEntries(
				Object.entries(value).map(([key, value]) => [
					key,
					Value.toSyscall(value),
				]),
			);
			return {
				kind: "object",
				value: syscallValue,
			};
		} else {
			return unreachable();
		}
	};

	export let fromSyscall = (value: syscall.Value): Value => {
		switch (value.kind) {
			case "null": {
				return undefined;
			}
			case "bool": {
				return value.value;
			}
			case "number": {
				return value.value;
			}
			case "string": {
				return value.value;
			}
			case "bytes": {
				return value.value;
			}
			case "relpath": {
				return Relpath.fromSyscall(value.value);
			}
			case "subpath": {
				return Subpath.fromSyscall(value.value);
			}
			case "blob": {
				return Blob.fromSyscall(value.value);
			}
			case "artifact": {
				return Artifact.fromSyscall(value.value);
			}
			case "placeholder": {
				return Placeholder.fromSyscall(value.value);
			}
			case "template": {
				return Template.fromSyscall(value.value);
			}
			case "operation": {
				return Operation.fromSyscall(value.value);
			}
			case "array": {
				return value.value.map((value) => Value.fromSyscall(value));
			}
			case "object": {
				return Object.fromEntries(
					Object.entries(value.value).map(([key, value]) => [
						key,
						Value.fromSyscall(value),
					]),
				);
			}
			default: {
				return unreachable();
			}
		}
	};
}
