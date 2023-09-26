import { assert as assert_ } from "./assert.ts";
import { Blob } from "./blob.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Package } from "./package.ts";
import { Placeholder } from "./placeholder.ts";
import { Symlink } from "./symlink.ts";
import { Task } from "./task.ts";
import { Template } from "./template.ts";

export type Value =
	| undefined
	| boolean
	| number
	| string
	| Uint8Array
	| Blob
	| Directory
	| File
	| Symlink
	| Placeholder
	| Template
	| Package
	| Task
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
			value instanceof Blob ||
			value instanceof Directory ||
			value instanceof File ||
			value instanceof Symlink ||
			value instanceof Placeholder ||
			value instanceof Template ||
			value instanceof Package ||
			value instanceof Task ||
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
}
