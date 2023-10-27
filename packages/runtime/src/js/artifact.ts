import { assert as assert_ } from "./assert.ts";
import { Blob } from "./blob.ts";
import { Directory } from "./directory.ts";
import { File } from "./file.ts";
import { Symlink } from "./symlink.ts";
import * as syscall from "./syscall.ts";

export type Artifact = Directory | File | Symlink;

export namespace Artifact {
	export type Id = string;

	export let is = (value: unknown): value is Artifact => {
		return Directory.is(value) || File.is(value) || Symlink.is(value);
	};

	export let expect = (value: unknown): Artifact => {
		assert_(is(value));
		return value;
	};

	export let assert = (value: unknown): asserts value is Artifact => {
		assert_(is(value));
	};

	export let archive = async (
		artifact: Artifact,
		format: Blob.ArchiveFormat,
	): Promise<Blob> => {
		return await syscall.archive(artifact, format);
	};
}
