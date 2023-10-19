import { Args } from "./args.ts";
import { Artifact } from "./artifact.ts";
import { assert as assert_, unreachable } from "./assert.ts";
import { Blob, blob } from "./blob.ts";
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
			contents: Array<Blob.Arg>;
			executable: Array<boolean>;
			references: Array<Artifact>;
		};
		let {
			contents: contentsArgs,
			executable: executableArgs,
			references,
		} = await Args.apply<File.Arg, Apply>(args, async (arg) => {
			if (Blob.Arg.is(arg)) {
				return { contents: { kind: "append" as const, value: arg } };
			} else if (File.is(arg)) {
				let contents = {
					kind: "append" as const,
					value: await arg.contents(),
				};
				let executable = {
					kind: "append" as const,
					value: await arg.executable(),
				};
				let references = {
					kind: "append" as const,
					value: await arg.references(),
				};
				return {
					contents,
					executable,
					references,
				};
			} else if (typeof arg === "object") {
				let object: Args.MutationObject<Apply> = {};
				if ("contents" in arg) {
					object.contents = {
						kind: "append" as const,
						value: arg.contents,
					};
				}
				if ("executable" in arg) {
					object.executable = {
						kind: "append" as const,
						value: arg.executable,
					};
				}
				if ("references" in arg) {
					object.references = {
						kind: "append" as const,
						value: arg.references,
					};
				}
				return object;
			} else {
				return unreachable();
			}
		});
		let contents = await blob(contentsArgs);
		let executable = (executableArgs ?? []).some((executable) => executable);
		references ??= [];
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
	export type Arg = Blob.Arg | File | ArgObject;

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
