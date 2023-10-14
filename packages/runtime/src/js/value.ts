import { assert as assert_ } from "./assert.ts";
import { Blob } from "./blob.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Package } from "./package.ts";
import { Symlink } from "./symlink.ts";
import { Target } from "./target.ts";
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
	| Template
	| Package
	| Target
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
			value instanceof Template ||
			value instanceof Package ||
			value instanceof Target ||
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
