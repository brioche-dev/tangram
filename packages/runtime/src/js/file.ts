import { Artifact } from "./artifact.ts";
import { assert as assert_, unreachable } from "./assert.ts";
import { Blob, blob } from "./blob.ts";
import { Args, MutationMap, apply, mutation } from "./mutation.ts";
import { Object_ } from "./object.ts";

export let file = async (...args: Args<File.Arg>) => {
	return await File.new(...args);
};

export class File {
	#handle: Object_.Handle;

	constructor(handle: Object_.Handle) {
		this.#handle = handle;
	}

	static withId(id: File.Id): File {
		return new File(Object_.Handle.withId(id));
	}

	static async new(...args: Args<File.Arg>): Promise<File> {
		type Apply = {
			contents?: Array<Blob.Arg>;
			executable?: Array<boolean>;
			references?: Array<Artifact>;
		};
		let {
			contents: contents_,
			executable: executable_,
			references: references_,
		} = await apply<File.Arg, Apply>(args, async (arg) => {
			if (arg === undefined) {
				return {};
			} else if (
				typeof arg === "string" ||
				arg instanceof Uint8Array ||
				Blob.is(arg)
			) {
				return {
					contents: await mutation({ kind: "array_append", value: [arg] }),
				};
			} else if (File.is(arg)) {
				return {
					contents: await mutation({
						kind: "array_append",
						value: [await arg.contents()],
					}),
					executable: await mutation({
						kind: "array_append",
						value: [await arg.executable()],
					}),
					references: await mutation({
						kind: "array_append",
						value: [await arg.references()],
					}),
				};
			} else if (typeof arg === "object") {
				let ret: Partial<MutationMap<Apply>> = {};
				if (arg.contents !== undefined) {
					ret.contents = await mutation({
						kind: "array_append",
						value: [arg.contents],
					});
				}
				if (arg.executable !== undefined) {
					ret.executable = await mutation({
						kind: "array_append",
						value: [arg.executable],
					});
				}
				if (arg.references !== undefined) {
					ret.references = await mutation({
						kind: "array_append",
						value: [arg.references],
					});
				}
				return ret;
			} else {
				return unreachable();
			}
		});
		let contents = await blob(contents_);
		let executable = (executable_ ?? []).some((executable) => executable);
		let references = references_ ?? [];
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

	get handle(): Object_.Handle {
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
	export type Arg =
		| undefined
		| string
		| Uint8Array
		| Blob
		| File
		| ArgObject
		| Array<Arg>;

	export type ArgObject = {
		contents?: Blob.Arg;
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
