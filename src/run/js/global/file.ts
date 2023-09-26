import { Artifact } from "./artifact.ts";
import { assert as assert_, unreachable } from "./assert.ts";
import { Blob, blob } from "./blob.ts";
import { Object_ } from "./object.ts";
import { Unresolved, resolve } from "./resolve.ts";
import { MaybeNestedArray, flatten } from "./util.ts";

export let file = async (...args: Array<Unresolved<File.Arg>>) => {
	return await File.new(...args);
};

export class File {
	#handle: Object_.Handle;

	constructor(handle: Object_.Handle) {
		this.#handle = handle;
	}

	static async new(...args: Array<Unresolved<File.Arg>>): Promise<File> {
		let {
			contents: contentsArgs,
			executable,
			references,
		} = flatten(
			await Promise.all(
				args.map(async function map(
					unresolvedArg: Unresolved<File.Arg>,
				): Promise<
					MaybeNestedArray<{
						contents: Blob.Arg;
						executable?: boolean;
						references?: Array<Artifact>;
					}>
				> {
					let arg = await resolve(unresolvedArg);
					if (Blob.Arg.is(arg)) {
						return { contents: arg };
					} else if (File.is(arg)) {
						return {
							contents: await arg.contents(),
							executable: await arg.executable(),
							references: await arg.references(),
						};
					} else if (arg instanceof Array) {
						return await Promise.all(arg.map(map));
					} else if (arg instanceof Object) {
						return {
							contents: arg.contents,
							executable: arg.executable,
							references: arg.references,
						};
					} else {
						return unreachable();
					}
				}),
			),
		).reduce<{
			contents: Array<Blob.Arg>;
			executable: boolean;
			references: Array<Artifact>;
		}>(
			(value, { contents, executable, references }) => {
				value.contents.push(contents);
				value.executable =
					executable !== undefined ? executable : value.executable;
				value.references.push(...(references ?? []));
				return value;
			},
			{ contents: [], executable: false, references: [] },
		);
		let contents = await blob(...contentsArgs);
		return new File(
			Object_.Handle.withObject({
				kind: "file",
				value: { contents, executable, references },
			}),
		);
	}

	static is(value: unknown): value is File {
		return value instanceof File;
	}

	static expect(value: unknown): File {
		assert_(File.is(value));
		return value;
	}

	static assert(value: unknown): asserts value is File {
		assert_(File.is(value));
	}

	async id(): Promise<File.Id> {
		return (await this.#handle.id()) as File.Id;
	}

	async object(): Promise<File.Object_> {
		let object = await this.#handle.object();
		assert_(object.kind === "file");
		return object.value;
	}

	handle(): Object_.Handle {
		return this.#handle;
	}

	async contents(): Promise<Blob> {
		return (await this.object()).contents;
	}

	async executable(): Promise<boolean> {
		return (await this.object()).executable;
	}

	async references(): Promise<Array<Artifact>> {
		return (await this.object()).references;
	}

	async size(): Promise<number> {
		return (await this.contents()).size();
	}

	async bytes(): Promise<Uint8Array> {
		return (await this.contents()).bytes();
	}

	async text(): Promise<string> {
		return (await this.contents()).text();
	}
}

export namespace File {
	export type Arg = Blob.Arg | File | Array<Arg> | ArgObject;

	export type ArgObject = {
		contents: Blob.Arg;
		executable?: boolean;
		references?: Array<Artifact>;
	};

	export type Id = string;

	export type Object_ = {
		contents: Blob;
		executable: boolean;
		references: Array<Artifact>;
	};
}
