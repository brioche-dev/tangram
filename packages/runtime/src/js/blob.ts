import { Args } from "./args.ts";
import { Artifact } from "./artifact.ts";
import { assert as assert_, unreachable } from "./assert.ts";
import { Checksum } from "./checksum.ts";
import * as encoding from "./encoding.ts";
import { Object_ } from "./object.ts";
import * as syscall from "./syscall.ts";

export let blob = async (...args: Args<Blob.Arg>) => {
	return await Blob.new(...args);
};

export let download = async (
	url: string,
	checksum: Checksum,
): Promise<Blob> => {
	return await Blob.download(url, checksum);
};

export class Blob {
	#handle: Object_.Handle;

	constructor(handle: Object_.Handle) {
		this.#handle = handle;
	}

	static withId(id: Blob.Id): Blob {
		return new Blob(Object_.Handle.withId(id));
	}

	static async new(...args: Args<Blob.Arg>): Promise<Blob> {
		type Apply = { children: Array<Uint8Array> };
		let { children } = await Args.apply<Blob.Arg, Apply>(args, async (arg) => {
			if (arg === undefined) {
				return { children: [] };
			} else if (typeof arg === "string") {
				return {
					children: {
						kind: "append" as const,
						value: encoding.utf8.encode(arg),
					},
				};
			} else if (arg instanceof Uint8Array) {
				return {
					children: {
						kind: "append" as const,
						value: arg,
					},
				};
			} else if (Blob.is(arg)) {
				return {
					children: { kind: "append" as const, value: await arg.bytes() },
				};
			} else {
				return unreachable();
			}
		});
		if (!children || children.length === 0) {
			children = [new Uint8Array()];
		}
		return new Blob(
			Object_.Handle.withObject({
				kind: "blob",
				value: await Promise.all(
					children.map<Promise<[Blob, number]>>(async (child) => {
						let childBlob = new Blob(
							Object_.Handle.withObject({
								kind: "blob",
								value: child,
							}),
						);
						return [childBlob, await childBlob.size()];
					}),
				),
			}),
		);
	}

	static async download(url: string, checksum: Checksum): Promise<Blob> {
		return await syscall.download(url, checksum);
	}

	static is(value: unknown): value is Blob {
		return value instanceof Blob;
	}

	static expect(value: unknown): Blob {
		assert_(Blob.is(value));
		return value;
	}

	static assert(value: unknown): asserts value is Blob {
		assert_(Blob.is(value));
	}

	async id(): Promise<Blob.Id> {
		return (await this.#handle.id()) as Blob.Id;
	}

	async object(): Promise<Blob.Object_> {
		let object = await this.#handle.object();
		assert_(object.kind === "blob");
		return object.value;
	}

	get handle(): Object_.Handle {
		return this.#handle;
	}

	async size(): Promise<number> {
		let object = await this.object();
		if (object instanceof Array) {
			return object.map(([_, size]) => size).reduce((a, b) => a + b, 0);
		} else {
			return object.byteLength;
		}
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

export namespace Blob {
	export type Arg = undefined | string | Uint8Array | Blob;

	export namespace Arg {
		export let is = (value: unknown): value is Arg => {
			return (
				value === undefined ||
				typeof value === "string" ||
				value instanceof Uint8Array ||
				Blob.is(value) ||
				(value instanceof Array && value.every(Arg.is))
			);
		};

		export let expect = (value: unknown): Arg => {
			assert_(is(value));
			return value;
		};

		export let assert = (value: unknown): asserts value is Arg => {
			assert_(is(value));
		};
	}

	export type Id = string;

	export type Object_ = Array<[Blob, number]> | Uint8Array;

	export type ArchiveFormat = ".tar" | ".zip";

	export type CompressionFormat = ".bz2" | ".gz" | ".lz" | ".xz" | ".zstd";
}
