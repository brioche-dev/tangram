import { Artifact } from "./artifact.ts";
import { assert as assert_, unreachable } from "./assert.ts";
import { Blob } from "./blob.ts";
import * as encoding from "./encoding.ts";
import { Args, apply, mutation } from "./mutation.ts";
import { Object_ } from "./object.ts";
import * as syscall from "./syscall.ts";

export let branch = async (...args: Args<Branch.Arg>): Promise<Branch> => {
	return await Branch.new(...args);
};

export class Branch {
	#handle: Object_.Handle;

	constructor(handle: Object_.Handle) {
		this.#handle = handle;
	}

	static withId(id: Branch.Id): Branch {
		return new Branch(Object_.Handle.withId(id));
	}

	static async new(...args: Args<Branch.Arg>): Promise<Branch> {
		type Apply = {
			children: Array<[Blob, number]>;
		};
		let { children } = await apply<Branch.Arg, Apply>(args, async (arg) => {
			if (arg === undefined) {
				return {};
			} else if (Branch.is(arg)) {
				return {
					children: await mutation({
						kind: "array_append",
						value: [[await arg.id(), await arg.size()]],
					}),
				};
			} else if (typeof arg === "object") {
				return {
					children: await mutation({
						kind: "array_append",
						value: arg.children ?? [],
					}),
				};
			} else {
				return unreachable();
			}
		});
		children ??= [];
		return new Branch(
			Object_.Handle.withObject({ kind: "branch", value: { children } }),
		);
	}

	static is(value: unknown): value is Branch {
		return value instanceof Branch;
	}

	static expect(value: unknown): Branch {
		assert_(Branch.is(value));
		return value;
	}

	static assert(value: unknown): asserts value is Branch {
		assert_(Branch.is(value));
	}

	async id(): Promise<Branch.Id> {
		return (await this.#handle.id()) as Branch.Id;
	}

	async object(): Promise<Branch.Object_> {
		let object = await this.#handle.object();
		assert_(object.kind === "branch");
		return object.value;
	}

	get handle(): Object_.Handle {
		return this.#handle;
	}

	async children(): Promise<Array<[Blob, number]>> {
		return (await this.object()).children;
	}

	async size(): Promise<number> {
		return (await this.children())
			.map(([_, size]) => size)
			.reduce((a, b) => a + b, 0);
	}

	async bytes(): Promise<Uint8Array> {
		return await syscall.read(this);
	}

	async text(): Promise<string> {
		return encoding.utf8.decode(await syscall.read(this));
	}

	async decompress(format: Blob.CompressionFormat): Promise<Blob> {
		return await syscall.decompress(this, format);
	}

	async extract(format: Blob.ArchiveFormat): Promise<Artifact> {
		return await syscall.extract(this, format);
	}
}

export namespace Branch {
	export type Arg = undefined | Branch | ArgObject | Array<Arg>;

	export type ArgObject = {
		children?: Array<[Blob, number]>;
	};

	export type Id = string;

	export type Object_ = { children: Array<[Blob, number]> };
}
