import { Artifact } from "./artifact.ts";
import { assert as assert_ } from "./assert.ts";
import { Blob } from "./blob.ts";
import { Block } from "./block.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Operation } from "./operation.ts";
import { Relpath, Subpath } from "./path.ts";
import { Placeholder } from "./placeholder.ts";
import { Resource } from "./resource.ts";
import { Symlink } from "./symlink.ts";
import { Target } from "./target.ts";
import { Task } from "./task.ts";
import { Template } from "./template.ts";

export type Value =
	| undefined
	| boolean
	| number
	| string
	| Uint8Array
	| Relpath
	| Subpath
	| Block
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
			value instanceof Block ||
			value instanceof Directory ||
			value instanceof File ||
			value instanceof Symlink ||
			value instanceof Placeholder ||
			value instanceof Template ||
			value instanceof Resource ||
			value instanceof Target ||
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
