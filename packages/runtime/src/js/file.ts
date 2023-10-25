import { Artifact } from "./artifact.ts";
import { assert as assert_, unreachable } from "./assert.ts";
import { Blob, blob } from "./blob.ts";
import {
	Args,
	MaybeMutationMap,
	Mutation,
	MutationMap,
	apply,
	mutation,
} from "./mutation.ts";
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
			contents?: Blob.Arg;
			executable?: boolean;
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
				arg instanceof Blob
			) {
				return { contents: arg };
			} else if (File.is(arg)) {
				return {
					contents: await arg.contents(),
					executable: await arg.executable(),
					references: await arg.references(),
				};
			} else if (arg instanceof Array) {
				let f = await File.new(...arg);
				return {
					contents: await f.contents(),
					executable: await f.executable(),
					references: await f.references(),
				};
			} else if (typeof arg === "object") {
				return arg;
			} else {
				return unreachable();
			}
		});
		let contents = await blob(contents_);
		let executable = executable_ ?? false;
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

let func = async () => {
	let f1 = File.new("hello", {
		references: [File.new("ref1")],
	});

	let fileArg = {
		references: [File.new("ref1")],
	};
	let f2 = File.new("hello", fileArg);

	let refMutation = await mutation({
		kind: "append_array" as const,
		value: [File.new("ref1")],
	});

	let refMutationTyped = await mutation<Array<Artifact>>({
		kind: "append_array" as const,
		value: [File.new("ref1")],
	});
	let fileArgMutation = {
		references: refMutation,
	};

	let fileArgMutationTyped: MaybeMutationMap<File.ArgObject> = {
		references: refMutationTyped,
	};

	let f3 = File.new("hello", fileArgMutationTyped);
};
