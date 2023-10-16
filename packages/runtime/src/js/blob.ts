import { Artifact } from "./artifact.ts";
import { assert as assert_, unreachable } from "./assert.ts";
import { Checksum } from "./checksum.ts";
import * as encoding from "./encoding.ts";
import { Object_ } from "./object.ts";
import { Unresolved, resolve } from "./resolve.ts";
import * as syscall from "./syscall.ts";
import { MaybeNestedArray, flatten } from "./util.ts";

export let blob = async (...args: Array<Unresolved<Blob.Arg>>) => {
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

	static async new(...args: Array<Unresolved<Blob.Arg>>): Promise<Blob> {
		let children = flatten(
			await Promise.all(
				args.map(async function map(
					unresolvedArg: Unresolved<Blob.Arg>,
				): Promise<MaybeNestedArray<Blob>> {
					let arg = await resolve(unresolvedArg);
					if (arg === undefined) {
						return [];
					} else if (typeof arg === "string") {
						return new Blob(
							Object_.Handle.withObject({
								kind: "blob",
								value: encoding.utf8.encode(arg),
							}),
						);
					} else if (arg instanceof Uint8Array) {
						return new Blob(
							Object_.Handle.withObject({ kind: "blob", value: arg }),
						);
					} else if (Blob.is(arg)) {
						return arg;
					} else if (arg instanceof Array) {
						return await Promise.all(arg.map(map));
					} else {
						return unreachable();
					}
				}),
			),
		);
		if (children.length === 0) {
			return new Blob(
				Object_.Handle.withObject({ kind: "blob", value: new Uint8Array() }),
			);
		} else if (children.length === 1) {
			return children[0]!;
		} else {
			return new Blob(
				Object_.Handle.withObject({
					kind: "blob",
					value: await Promise.all(
						children.map<Promise<[Blob, number]>>(async (child) => {
							return [child, await child.size()];
						}),
					),
				}),
			);
		}
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
	export type Arg = undefined | string | Uint8Array | Blob | Array<Arg>;

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
