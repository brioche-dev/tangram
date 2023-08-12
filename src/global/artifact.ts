import { assert as assert_ } from "./assert.ts";
import { Block } from "./block.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Symlink } from "./symlink.ts";
import * as syscall from "./syscall.ts";

export type Artifact = Directory | File | Symlink;

export namespace Artifact {
	export let is = (value: unknown): value is Artifact => {
		return (
			value instanceof Directory ||
			value instanceof File ||
			value instanceof Symlink
		);
	};

	export let expect = (value: unknown): Artifact => {
		assert_(is(value));
		return value;
	};

	export let assert = (value: unknown): asserts value is Artifact => {
		assert_(is(value));
	};

	export let withBlock = async (block: Block): Promise<Artifact> => {
		return await syscall.artifact.get(block);
	};
}
